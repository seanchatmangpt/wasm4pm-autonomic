#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    #[test]
    fn test_ralph_orchestrator_flow() {
        // 1. Create a temporary directory to avoid touching project root
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let temp_path = temp_dir.path();

        // 2. Setup test environment in the temp dir
        let ideas_path = temp_path.join("IDEAS.md");
        fs::write(&ideas_path, "Test Improvement: Add doc comments to bitset\n")
            .expect("Failed to write IDEAS.md");

        // 3. Run ralph in test mode within the temp directory
        let output = Command::new("cargo")
            .args(["run", "--bin", "ralph", "--", "--test"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to execute ralph");

        assert!(
            output.status.success(),
            "Ralph execution failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // 4. Verify directory structure (in temp dir)
        let wreckit_dir = temp_path.join(".wreckit");
        assert!(wreckit_dir.exists(), ".wreckit directory should exist");

        // Find the idea directory (slug might vary slightly)
        let entries = fs::read_dir(&wreckit_dir).expect("Failed to read .wreckit");
        let mut found_idea = false;
        for entry in entries {
            let path = entry.unwrap().path();
            if path.to_str().unwrap().contains("add-doc-comments") {
                found_idea = true;
                assert!(path.join("research.md").exists(), "research.md missing");
                assert!(path.join("plan.md").exists(), "plan.md missing");
                assert!(path.join("implement.md").exists(), "implement.md missing");
            }
        }
        assert!(found_idea, "Idea directory not found in .wreckit");

        // 5. Verify hook injection (in temp dir)
        assert!(
            temp_path.join(".gemini/settings.json").exists(),
            "settings.json missing"
        );
        assert!(
            temp_path.join(".gemini/hooks/supervisor.sh").exists(),
            "supervisor.sh missing"
        );

        // TempDir automatically cleans up on drop
    }
}

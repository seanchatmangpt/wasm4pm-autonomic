#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    #[test]
    fn test_ralph_orchestrator_flow() {
        // 1. Setup a clean environment
        let ideas_path = Path::new("IDEAS_TEST.md");
        fs::write(ideas_path, "Test Improvement: Add doc comments to bitset\n").expect("Failed to write IDEAS_TEST.md");
        
        // 2. Run ralph in test mode
        // We use a temporary IDEAS.md for the main script
        fs::copy(ideas_path, "IDEAS.md").expect("Failed to copy IDEAS.md");
        
        let output = Command::new("cargo")
            .args(["run", "--bin", "ralph", "--", "--test"])
            .output()
            .expect("Failed to execute ralph");
        
        assert!(output.status.success(), "Ralph execution failed: {}", String::from_utf8_lossy(&output.stderr));
        
        // 3. Verify directory structure
        let wreckit_dir = Path::new(".wreckit");
        assert!(wreckit_dir.exists(), ".wreckit directory should exist");
        
        // Find the idea directory (slug might vary slightly)
        let entries = fs::read_dir(wreckit_dir).expect("Failed to read .wreckit");
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
        
        // 4. Verify hook injection
        assert!(Path::new(".gemini/settings.json").exists(), "settings.json missing");
        assert!(Path::new(".gemini/hooks/supervisor.sh").exists(), "supervisor.sh missing");
        
        // 5. Cleanup
        fs::remove_file("IDEAS_TEST.md").ok();
    }
}

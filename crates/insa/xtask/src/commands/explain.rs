use anyhow::{anyhow, Result};
use insa_instinct::{InstinctByte, KappaByte};

pub fn execute(family: String, value: u8) -> Result<()> {
    match family.to_lowercase().as_str() {
        "inst8" | "instinct" => {
            let inst = InstinctByte(value);
            println!("INST8 0b{:08b}", value);
            if inst.contains(InstinctByte::SETTLE) { println!("  Settle"); }
            if inst.contains(InstinctByte::RETRIEVE) { println!("  Retrieve"); }
            if inst.contains(InstinctByte::INSPECT) { println!("  Inspect"); }
            if inst.contains(InstinctByte::ASK) { println!("  Ask"); }
            if inst.contains(InstinctByte::AWAIT) { println!("  Await"); }
            if inst.contains(InstinctByte::REFUSE) { println!("  Refuse"); }
            if inst.contains(InstinctByte::ESCALATE) { println!("  Escalate"); }
            if inst.contains(InstinctByte::IGNORE) { println!("  Ignore"); }
        }
        "kappa8" | "kappa" => {
            let kap = KappaByte(value);
            println!("KAPPA8 0b{:08b}", value);
            if kap.contains(KappaByte::REFLECT) { println!("  Reflect / ELIZA"); }
            if kap.contains(KappaByte::PRECONDITION) { println!("  Precondition / STRIPS"); }
            if kap.contains(KappaByte::GROUND) { println!("  Ground / SHRDLU"); }
            if kap.contains(KappaByte::PROVE) { println!("  Prove / Prolog"); }
            if kap.contains(KappaByte::RULE) { println!("  Rule / MYCIN"); }
            if kap.contains(KappaByte::RECONSTRUCT) { println!("  Reconstruct / DENDRAL"); }
            if kap.contains(KappaByte::FUSE) { println!("  Fuse / HEARSAY-II"); }
            if kap.contains(KappaByte::REDUCE_GAP) { println!("  ReduceGap / GPS"); }
        }
        _ => return Err(anyhow!("Unknown family: {}. Supported: inst8, kappa8", family)),
    }
    Ok(())
}

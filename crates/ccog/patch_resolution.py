import sys

with open("../insa/insa-instinct/src/resolution.rs", "r") as f:
    content = f.read()

new_content = content.replace("""pub struct InstinctResolutionLut {
    pub selected_lut: [SelectedInstinctByte; 256],
    pub class_lut: [ResolutionClass; 256],
}

impl InstinctResolutionLut {
    pub const fn resolve(&self, activation: InstinctByte) -> InstinctResolution {
        let bits = activation.bits() as usize;
        InstinctResolution {
            activation,
            selected: self.selected_lut[bits],
            inhibited: InstinctByte(0), // Simplified for now
            conflict: ConflictStatus::Valid,
            class: self.class_lut[bits],
        }
    }
}""", """pub struct InstinctResolutionLut {
    pub selected_lut: [SelectedInstinctByte; 256],
    pub class_lut: [ResolutionClass; 256],
    pub inhibition_lut: [InstinctByte; 256],
    pub conflict_lut: [ConflictStatus; 256],
}

impl InstinctResolutionLut {
    pub const fn resolve(&self, activation: InstinctByte) -> InstinctResolution {
        let bits = activation.bits() as usize;
        InstinctResolution {
            activation,
            selected: self.selected_lut[bits],
            inhibited: self.inhibition_lut[bits],
            conflict: self.conflict_lut[bits],
            class: self.class_lut[bits],
        }
    }
}""")

with open("../insa/insa-instinct/src/resolution.rs", "w") as f:
    f.write(new_content)

// Shared camera RAW extension registry.
pub const RAW_EXTENSIONS: &[&str] = &[
    "crw", "cr2", "cr3", // Canon
    "nef", "nrw", "nrv", // Nikon
    "arw", "srf", "sr2", "sr1", "sr",  // Sony
    "raf", // Fujifilm
    "orf", "ori", "obm", // Olympus
    "rw2", "raw", // Panasonic
    "pef", "ptx", "pkx", // Pentax
    "3fr", "fff", // Hasselblad
    "iiq", "cap", "eip", // Phase One
    "dcr", "dcs", "drf", "k25", "kdc", "kqc", "kc2", // Kodak
    "rwl", "dng", // Leica (dng is shared, listed generically below too)
    "srw", // Samsung
    "x3f", // Sigma
    "mos", "mef", "mfw", // Leaf / Mamiya
    "erf", // Epson
    "gpr", // GoPro
    "rw1", "j6i", // Ricoh
    "bay", "cam", // Casio
    "ari", // ARRI
    "r3d", // RED
    "stx", "sti", // Sinar
    "pxn", // Logitech
    "mrw", "mdc", // Minolta
    "dng", "rwz", "cxi", "fpix", "rdc", "qtk", // Generic / Other (rawzor, foveon, etc)
];

pub fn is_raw_extension(ext: &str) -> bool {
    RAW_EXTENSIONS
        .iter()
        .any(|raw_ext| raw_ext.eq_ignore_ascii_case(ext))
}

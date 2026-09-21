// iOS EXTRACTOR
pub enum IosMethod {
    LogicalBackup, // iTunes backup via libimobiledevice
    AdvancedLogical, // House Arrest
    Checkm8, // iPhone 5s à X uniquement, DFU mode
}

pub fn extract_ios(method: IosMethod) {
    // Toujours avec code ou autorisation
}

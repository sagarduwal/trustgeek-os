//! Bootloader and application information reading
//!
//! Functions to read app descriptor and partition information

/// Application information structure
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
}

/// Get application descriptor information
///
/// Returns AppInfo with app name and version
pub fn get_app_info() -> AppInfo {
    // Access the app descriptor created by esp_app_desc!() macro
    // The macro creates app descriptor data that we can access
    // For now, use default values - will be enhanced when we determine the exact API
    AppInfo {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
    }
}

/// Partition information structure
pub struct PartitionInfo {
    pub name: &'static str,
    pub size: &'static str,
}

/// Get partition information
///
/// Returns an array of partition info (limited to 4 for display purposes)
pub fn get_partition_info() -> [PartitionInfo; 4] {
    // TODO: Implement partition reading from esp-bootloader-esp-idf
    // For now, return placeholder data
    [
        PartitionInfo {
            name: "app",
            size: "1MB",
        },
        PartitionInfo {
            name: "data",
            size: "512KB",
        },
        PartitionInfo {
            name: "ota_0",
            size: "1MB",
        },
        PartitionInfo {
            name: "ota_1",
            size: "1MB",
        },
    ]
}

/// Convert partition info to tuple format for OLED display
pub fn partition_info_to_tuples(partitions: &[PartitionInfo; 4]) -> [(&str, &str); 4] {
    [
        (partitions[0].name, partitions[0].size),
        (partitions[1].name, partitions[1].size),
        (partitions[2].name, partitions[2].size),
        (partitions[3].name, partitions[3].size),
    ]
}

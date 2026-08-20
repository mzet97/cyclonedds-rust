const EXPECTED_VERSION: &str = "11.0.1";

#[test]
fn bundled_source_announces_the_audited_cyclonedds_version() {
    // Given: every version-bearing file shipped by cyclonedds-src.
    let announcements = [
        include_str!("../src/cyclonedds/CMakeLists.txt"),
        include_str!("../src/cyclonedds/package.xml"),
        include_str!("../src/cyclonedds/src/core/cdr/CMakeLists.txt"),
    ];

    // When: the package metadata is checked against the audited source release.
    let all_match = announcements
        .iter()
        .all(|announcement| announcement.contains(EXPECTED_VERSION));

    // Then: no bundled source component can announce the former release.
    assert!(all_match, "bundled CycloneDDS source version drifted");
}

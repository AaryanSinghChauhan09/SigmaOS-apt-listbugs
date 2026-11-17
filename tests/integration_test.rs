// Integration test for SigmaOS-apt-listbugs
// Verifies the crate builds and links as a dependency.
#![allow(unused_imports)]

use apt_listbugs as _;

#[test]
fn crate_links_and_runs() {
    assert_eq!(2 + 2, 4);
}

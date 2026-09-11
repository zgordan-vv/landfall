#![allow(missing_docs)]

use landfall_storage::SeedIds;

#[test]
fn seed_ids_are_stable_and_distinct() {
    let ids = SeedIds {
        project_id: uuid::Uuid::from_u128(0x018f_2d8e_7b3a_7c01_8a01_0000_0000_0001),
        environment_id: uuid::Uuid::from_u128(0x018f_2d8e_7b3a_7c01_8a01_0000_0000_0002),
    };
    assert_ne!(ids.project_id, ids.environment_id);
    assert_eq!(ids.project_id.get_version_num(), 7);
}

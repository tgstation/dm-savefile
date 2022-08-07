#[test]
fn mothblocks() {
    insta::assert_yaml_snapshot!(dm_savefile::extract_savefile(
        &include_bytes!("./savefiles/mothblocks.sav")[..]
    )
    .unwrap());
}

#[test]
fn bebeyoshi() {
    insta::assert_yaml_snapshot!(dm_savefile::extract_savefile(
        &include_bytes!("./savefiles/bebeyoshi.sav")[..]
    )
    .unwrap());
}

#[test]
fn bdudy() {
    insta::assert_yaml_snapshot!(dm_savefile::extract_savefile(
        &include_bytes!("./savefiles/bdudy.sav")[..]
    )
    .unwrap());
}

#[test]
fn lemonboye() {
    insta::assert_yaml_snapshot!(dm_savefile::extract_savefile(
        &include_bytes!("./savefiles/lemonboye.sav")[..]
    )
    .unwrap());
}

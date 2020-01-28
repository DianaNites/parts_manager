#[derive(Debug)]
pub struct Disk {}

pub fn get_disks() -> Vec<(String, Disk)> {
    vec!["Test", "Example", "Thing"]
        .into_iter()
        .map(|s| ("Disk ".to_string() + s.into(), Disk {}))
        .collect()
}

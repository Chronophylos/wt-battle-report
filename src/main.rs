fn main() {
    for entry in std::fs::read_dir("./data").unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            continue;
        }

        let content = std::fs::read_to_string(&path).unwrap();
        let battle_report = wt_battle_report::from_str(&content).unwrap();
        println!("{:#?}", battle_report);
    }
}

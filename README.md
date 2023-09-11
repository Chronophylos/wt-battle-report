# War Thunder Battle Report Parser

Recently Gaijin changed how battle reports are displayed in your message log.
Doing that, they added the ability to copy a battle report to your clipboard.
This library can deserialize that battle report using serde.

## Usage

Add the following to your `Cargo.toml`:

```toml
[dependencies]
wt-battle-report = "0.1"
```

Then you can use the library like this:

```rust
fn main() {
    let report = "Battle report text";
    let battle_report = wt_battle_report::from_str(report).unwrap();
    println!("{:#?}", battle_report);
}
```

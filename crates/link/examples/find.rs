// SPDX-License-Identifier: GPL-2.0-or-later
//! Lists the scopes that answer on the local network: `cargo run -p oos-link --example find`.
fn main() {
    match oos_link::discover(std::time::Duration::from_secs(2)) {
        Ok(v) if v.is_empty() => println!("no scope answered"),
        Ok(v) => v.iter().for_each(|f| println!("{}  {}", f.endpoint, f.name)),
        Err(e) => eprintln!("{e}"),
    }
}

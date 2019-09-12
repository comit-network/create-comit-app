// pub fn start_cnd() {
//     Command::new("cnd").spawn().expect("cnd not found in path");
// }

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest;

    #[test]
    fn can_ping_cnd() {
        let cnd = Cnd::start();

        let endpoint = format!("http://localhost:{}", cnd.port);
        assert!(reqwest::get(&endpoint).unwrap().status().is_success())
    }
}

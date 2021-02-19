use rustc_session::Session;

pub struct Conf {}

pub fn read_conf(_conf: &[&str], _sess: &Session) -> Result<Conf, String> {
    Ok(Conf {})
}

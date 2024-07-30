#[derive(Debug)]
pub enum PuddlerSettingsError {
  Corrupt,
}

#[derive(Debug)]
pub enum MediaCenterConfigError {
  Corrupt, // yeah lol I had planned more than just this but I guess it didn't really pan out
}

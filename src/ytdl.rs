use std::{io::BufRead, process::Stdio};

use url::Url;

pub async fn get_stream_url(url: &Url) -> Vec<Url> {
    let out = tokio::process::Command::new("/usr/bin/env")
        .arg("yt-dlp")
        .arg("-g")
        .arg(url.as_str())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .await
        .expect("failed to execute yt-dlp");
    log::debug!("{}", String::from_utf8_lossy(&out.stdout));
    out.stdout.lines().next();
    let stdout = String::from_utf8_lossy(&out.stdout);

    stdout
        .lines()
        .filter_map(|l| Url::try_from(l).ok())
        .collect()
}

extern crate sollet;

#[cfg(not(target_os = "android"))]
#[tokio::main]
async fn main() {
    sollet::desktop_main().await;
}

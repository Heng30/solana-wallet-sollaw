extern crate sollaw;

#[cfg(not(target_os = "android"))]
#[tokio::main]
async fn main() {
    sollaw::desktop_main().await;
}

fn main() {
    println!("Running cargo fmt...");
    let output = std::process::Command::new("cargo")
        .args(["fmt"])
        .output()
        .expect("Failed to execute cargo fmt");

    if output.status.success() {
        println!("Code formatting successful!");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("Error formatting code: {}", stderr);
    }
}

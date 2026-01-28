//! Debug test for NCBI FTP connection
//!
//! Run with: cargo test --test ncbi_ftp_debug_test -- --nocapture --ignored

use suppaftp::FtpStream;

#[test]
#[ignore]
fn test_ftp_connection_debug() {
    println!("\n=== Testing Raw FTP Connection ===\n");

    // Connect
    println!("Connecting to ftp.ncbi.nlm.nih.gov:21...");
    let mut ftp_stream = FtpStream::connect("ftp.ncbi.nlm.nih.gov:21").expect("Failed to connect");

    println!("Connected!");

    // Set passive mode
    ftp_stream.set_mode(suppaftp::Mode::ExtendedPassive);
    println!("Set to Extended Passive Mode");

    // Login
    println!("Logging in as anonymous...");
    ftp_stream
        .login("anonymous", "anonymous")
        .expect("Failed to login");
    println!("Logged in!");

    // Try to list root taxonomy directory
    println!("\n--- Listing /pub/taxonomy/ ---");
    match ftp_stream.list(Some("/pub/taxonomy/")) {
        Ok(entries) => {
            println!("Found {} entries:", entries.len());
            for (i, entry) in entries.iter().enumerate().take(20) {
                println!("  [{}]: {}", i, entry);
            }
        },
        Err(e) => {
            println!("Failed to list: {}", e);
        },
    }

    // Try to list taxdump_archive directory
    println!("\n--- Listing /pub/taxonomy/taxdump_archive/ ---");
    match ftp_stream.list(Some("/pub/taxonomy/taxdump_archive/")) {
        Ok(entries) => {
            println!("Found {} entries:", entries.len());
            for (i, entry) in entries.iter().enumerate().take(20) {
                println!("  [{}]: {}", i, entry);
            }

            // Try to parse filenames
            println!("\nParsing filenames:");
            for entry in entries.iter().take(10) {
                if let Some(filename) = entry.split_whitespace().last() {
                    println!("  - {}", filename);
                }
            }
        },
        Err(e) => {
            println!("Failed to list: {}", e);
        },
    }

    // Try nlst command instead
    println!("\n--- Trying NLST /pub/taxonomy/taxdump_archive/ ---");
    match ftp_stream.nlst(Some("/pub/taxonomy/taxdump_archive/")) {
        Ok(names) => {
            println!("Found {} names:", names.len());
            for (i, name) in names.iter().enumerate().take(20) {
                println!("  [{}]: {}", i, name);
            }
        },
        Err(e) => {
            println!("Failed to nlst: {}", e);
        },
    }

    // Logout
    let _ = ftp_stream.quit();
    println!("\n✓ Test completed");
}

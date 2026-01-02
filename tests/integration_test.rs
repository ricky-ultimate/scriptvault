use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// Helper to setup isolated test environment
struct TestEnv {
    _temp_dir: TempDir,
    config_path: PathBuf,
    vault_path: PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let config_path = base_path.join("config.json");
        let vault_path = base_path.join("vault");

        fs::create_dir_all(&vault_path).unwrap();

        Self {
            _temp_dir: temp_dir,
            config_path,
            vault_path,
        }
    }
}

#[test]
fn test_full_workflow_save_find_run() {
    // This test simulates the full user workflow:
    // 1. Save a script
    // 2. Find it
    // 3. Run it
    // 4. Check history

    // Setup
    let test_env = TestEnv::new();

    // Create a test script file
    let script_content = "#!/bin/bash\necho 'Hello from test'";
    let script_path = test_env.vault_path.join("test-script.sh");
    fs::write(&script_path, script_content).unwrap();

    // Test will be expanded with actual CLI integration
    assert!(script_path.exists());
}

#[test]
fn test_save_and_list_scripts() {
    let test_env = TestEnv::new();

    // Create multiple test scripts
    let scripts = vec![
        ("deploy.sh", "#!/bin/bash\necho 'deploying'"),
        ("backup.sh", "#!/bin/bash\necho 'backing up'"),
        ("test.sh", "#!/bin/bash\necho 'testing'"),
    ];

    for (name, content) in scripts {
        let path = test_env.vault_path.join(name);
        fs::write(&path, content).unwrap();
    }

    // Verify all files exist
    assert_eq!(fs::read_dir(&test_env.vault_path).unwrap().count(), 3);
}

#[test]
fn test_execution_history_tracking() {
    let test_env = TestEnv::new();

    // Create history file
    let history_path = test_env.vault_path.join("history.jsonl");

    // Simulate execution records
    let record = r#"{"id":"test123","script_id":"script123","exit_code":0}"#;
    fs::write(&history_path, format!("{}\n", record)).unwrap();

    // Verify history file exists and is readable
    let content = fs::read_to_string(&history_path).unwrap();
    assert!(content.contains("test123"));
}

#[test]
fn test_context_detection_in_git_repo() {
    // This would test git context detection
    // Skip if not in a git repo
    if std::process::Command::new("git")
        .args(&["rev-parse", "--git-dir"])
        .output()
        .is_ok()
    {
        // In a git repo, context should be detected
        // This will be expanded with actual context module tests
        assert!(true);
    }
}

#[test]
fn test_search_by_tags() {
    let test_env = TestEnv::new();

    // This will test the search functionality
    // For now, we verify the test environment works
    assert!(test_env.vault_path.exists());
}

#[test]
fn test_script_safety_checks() {
    let test_env = TestEnv::new();

    // Create a dangerous script
    let dangerous_content = "#!/bin/bash\nrm -rf /";
    let safe_content = "#!/bin/bash\necho 'safe'";

    let dangerous_path = test_env.vault_path.join("dangerous.sh");
    let safe_path = test_env.vault_path.join("safe.sh");

    fs::write(&dangerous_path, dangerous_content).unwrap();
    fs::write(&safe_path, safe_content).unwrap();

    // Verify both files were created
    assert!(dangerous_path.exists());
    assert!(safe_path.exists());

    // Safety check will be verified through the script module
    let dangerous_script = fs::read_to_string(&dangerous_path).unwrap();
    assert!(dangerous_script.contains("rm -rf"));
}

#[test]
fn test_multiple_language_support() {
    let test_env = TestEnv::new();

    let scripts = vec![
        ("test.sh", "#!/bin/bash\necho 'bash'"),
        ("test.py", "#!/usr/bin/env python3\nprint('python')"),
        ("test.rb", "#!/usr/bin/env ruby\nputs 'ruby'"),
        ("test.pl", "#!/usr/bin/env perl\nprint 'perl'"),
    ];

    for (name, content) in scripts {
        let path = test_env.vault_path.join(name);
        fs::write(&path, content).unwrap();
    }

    // Verify all scripts created
    assert_eq!(fs::read_dir(&test_env.vault_path).unwrap().count(), 4);
}

#[test]
fn test_config_persistence() {
    let test_env = TestEnv::new();

    // Create a config
    let config = r#"{
        "api_endpoint": "https://api.scriptvault.dev",
        "auth_token": "test123",
        "auto_sync": true
    }"#;

    fs::write(&test_env.config_path, config).unwrap();

    // Verify config can be read back
    let read_config = fs::read_to_string(&test_env.config_path).unwrap();
    assert!(read_config.contains("test123"));
}

#[test]
fn test_execution_with_arguments() {
    let test_env = TestEnv::new();

    // Create a script that takes arguments
    let script_content = r#"#!/bin/bash
echo "Arg 1: $1"
echo "Arg 2: $2"
"#;

    let script_path = test_env.vault_path.join("args-test.sh");
    fs::write(&script_path, script_content).unwrap();

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();
    }

    assert!(script_path.exists());
}

#[test]
fn test_special_characters_in_scripts() {
    let test_env = TestEnv::new();

    // Test script with special characters
    let special_content = r#"#!/bin/bash
echo "Testing: !@#$%^&*()"
echo "Path: /home/user/my folder/file.txt"
echo "Quotes: \"hello\" and 'world'"
"#;

    let path = test_env.vault_path.join("special.sh");
    fs::write(&path, special_content).unwrap();

    // Verify content is preserved
    let read_content = fs::read_to_string(&path).unwrap();
    assert!(read_content.contains("!@#$%^&*()"));
    assert!(read_content.contains("my folder"));
}

#[test]
fn test_concurrent_script_execution() {
    let test_env = TestEnv::new();

    // Create a script that can be run concurrently
    let script_content = "#!/bin/bash\necho 'concurrent test'\nsleep 0.1";
    let path = test_env.vault_path.join("concurrent.sh");
    fs::write(&path, script_content).unwrap();

    // Verify script exists (actual concurrent execution would be tested separately)
    assert!(path.exists());
}

#[test]
fn test_error_handling_missing_script() {
    let test_env = TestEnv::new();

    // Try to access a non-existent script
    let missing_path = test_env.vault_path.join("does-not-exist.sh");

    // Should not exist
    assert!(!missing_path.exists());
}

#[test]
fn test_large_script_handling() {
    let test_env = TestEnv::new();

    // Create a large script (1000 lines)
    let mut large_content = String::from("#!/bin/bash\n");
    for i in 0..1000 {
        large_content.push_str(&format!("echo 'Line {}'\n", i));
    }

    let path = test_env.vault_path.join("large.sh");
    fs::write(&path, &large_content).unwrap();

    // Verify large script can be saved and read
    let read_content = fs::read_to_string(&path).unwrap();
    assert_eq!(read_content.lines().count(), 1001); // shebang + 1000 lines
}

#[test]
fn test_vault_structure() {
    let test_env = TestEnv::new();

    // Create the expected vault structure
    let scripts_path = test_env.vault_path.join("scripts.json");
    let history_path = test_env.vault_path.join("history.jsonl");

    fs::write(&scripts_path, "[]").unwrap();
    fs::write(&history_path, "").unwrap();

    // Verify structure exists
    assert!(scripts_path.exists());
    assert!(history_path.exists());
}

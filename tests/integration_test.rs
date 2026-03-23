use chrono::Utc;
use scriptvault::context::{contexts_match, normalize_git_url};
use scriptvault::script::{
    ExecutionRecord, Script, ScriptContext, ScriptLanguage, ScriptMetadata, SyncState, Visibility,
};
use scriptvault::storage::local::LocalStorage;
use scriptvault::storage::StorageBackend;
use std::collections::HashMap;
use tempfile::TempDir;

fn make_script(name: &str, content: &str) -> Script {
    Script {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        content: content.to_string(),
        version: "v1.0.0".to_string(),
        language: ScriptLanguage::Bash,
        tags: vec![],
        description: None,
        author: "test".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        context: ScriptContext {
            directory: None,
            git_repo: None,
            git_branch: None,
            environment: HashMap::new(),
        },
        metadata: ScriptMetadata {
            hash: uuid::Uuid::new_v4().to_string(),
            size_bytes: content.len(),
            line_count: content.lines().count(),
            use_count: 0,
            success_count: 0,
            failure_count: 0,
            last_run: None,
            last_run_by: None,
            avg_runtime_ms: None,
        },
        visibility: Visibility::Private,
        sync_state: SyncState::default(),
    }
}
fn storage(tmp: &TempDir) -> LocalStorage {
    LocalStorage::new(tmp.path().to_path_buf()).unwrap()
}

#[test]
fn test_save_and_retrieve_by_name() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    let script = make_script("deploy", "#!/bin/bash\necho deploy");
    s.save_script(&script).unwrap();
    let loaded = s.load_script_by_name("deploy").unwrap();
    assert_eq!(loaded.name, "deploy");
    assert_eq!(loaded.content, "#!/bin/bash\necho deploy");
}

#[test]
fn test_save_and_retrieve_by_id() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    let script = make_script("backup", "echo backup");
    let id = script.id.clone();
    s.save_script(&script).unwrap();
    let loaded = s.load_script(&id).unwrap();
    assert_eq!(loaded.id, id);
}

#[test]
fn test_resave_same_name_replaces_record() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    let script = make_script("deploy", "echo v1");
    s.save_script(&script).unwrap();

    let mut updated = make_script("deploy", "echo v2");
    updated.id = script.id.clone();
    s.save_script(&updated).unwrap();

    let all = s.list_scripts().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].content, "echo v2");
}

#[test]
fn test_resave_diverged_id_replaces_by_name() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    let original = make_script("deploy", "echo original");
    s.save_script(&original).unwrap();

    let diverged = make_script("deploy", "echo diverged");
    s.save_script(&diverged).unwrap();

    let all = s.list_scripts().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].content, "echo diverged");
    assert_eq!(all[0].id, diverged.id);
}

#[test]
fn test_update_preserves_id() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    let mut script = make_script("myscript", "echo old");
    s.save_script(&script).unwrap();
    script.content = "echo new".to_string();
    s.update_script(&script).unwrap();
    let loaded = s.load_script_by_name("myscript").unwrap();
    assert_eq!(loaded.content, "echo new");
    assert_eq!(loaded.id, script.id);
}

#[test]
fn test_update_unknown_id_errors() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    let ghost = make_script("ghost", "echo ghost");
    assert!(s.update_script(&ghost).is_err());
}

#[test]
fn test_delete_removes_script() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    let script = make_script("to-delete", "echo bye");
    let id = script.id.clone();
    s.save_script(&script).unwrap();
    s.delete_script(&id).unwrap();
    assert!(!s.script_exists(&id).unwrap());
    assert!(s.load_script_by_name("to-delete").is_err());
}

#[test]
fn test_delete_unknown_id_errors() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    assert!(s.delete_script("nonexistent-id").is_err());
}

#[test]
fn test_list_returns_all_scripts() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    s.save_script(&make_script("zebra", "echo z")).unwrap();
    s.save_script(&make_script("alpha", "echo a")).unwrap();
    s.save_script(&make_script("mango", "echo m")).unwrap();
    let scripts = s.list_scripts().unwrap();
    assert_eq!(scripts.len(), 3);
    let names: Vec<&str> = scripts.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"zebra"));
    assert!(names.contains(&"alpha"));
    assert!(names.contains(&"mango"));
}

#[test]
fn test_copy_creates_independent_script() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    let original = make_script("original", "echo original");
    let original_id = original.id.clone();
    s.save_script(&original).unwrap();

    let mut copy = original.clone();
    copy.id = uuid::Uuid::new_v4().to_string();
    copy.name = "copy".to_string();
    copy.metadata.use_count = 0;
    s.save_script(&copy).unwrap();

    assert_eq!(s.list_scripts().unwrap().len(), 2);
    assert_ne!(copy.id, original_id);

    let loaded_copy = s.load_script_by_name("copy").unwrap();
    assert_eq!(loaded_copy.content, "echo original");
    assert_eq!(loaded_copy.metadata.use_count, 0);
}

#[test]
fn test_rename_via_update() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    let mut script = make_script("old-name", "echo hi");
    s.save_script(&script).unwrap();
    script.name = "new-name".to_string();
    s.update_script(&script).unwrap();
    assert!(s.load_script_by_name("old-name").is_err());
    assert!(s.load_script_by_name("new-name").is_ok());
}

#[test]
fn test_rename_then_resave_original_name_creates_new_entry() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);

    let mut original = make_script("deploy", "echo deploy v1");
    let original_id = original.id.clone();
    s.save_script(&original).unwrap();

    original.name = "deploy-old".to_string();
    s.update_script(&original).unwrap();

    let new_deploy = make_script("deploy", "echo deploy v2");
    s.save_script(&new_deploy).unwrap();

    let all = s.list_scripts().unwrap();
    assert_eq!(all.len(), 2);

    let renamed = s.load_script_by_name("deploy-old").unwrap();
    assert_eq!(renamed.id, original_id);
    assert_eq!(renamed.content, "echo deploy v1");

    let fresh = s.load_script_by_name("deploy").unwrap();
    assert_ne!(fresh.id, original_id);
    assert_eq!(fresh.content, "echo deploy v2");
}

#[test]
fn test_script_exists() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    let script = make_script("exists-test", "echo hi");
    let id = script.id.clone();
    assert!(!s.script_exists(&id).unwrap());
    s.save_script(&script).unwrap();
    assert!(s.script_exists(&id).unwrap());
}

#[test]
fn test_metadata_totals() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    s.save_script(&make_script("s1", "echo s1")).unwrap();
    s.save_script(&make_script("s2", "echo s2")).unwrap();
    s.save_script(&make_script("s3", "echo s3")).unwrap();
    let meta = s.get_metadata().unwrap();
    assert_eq!(meta.total_scripts, 3);
    assert_eq!(meta.backend_type, "local");
    assert!(meta.total_size_bytes > 0);
}

#[test]
fn test_health_check_on_valid_vault() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    assert!(s.health_check().unwrap());
}

#[test]
fn test_script_is_safe_clean_content() {
    let script = Script::new(
        "safe".to_string(),
        "echo hello\nls -la\ngit status".to_string(),
        ScriptLanguage::Bash,
    );
    assert!(script.is_safe());
}

#[test]
fn test_script_is_not_safe_rm_rf() {
    let script = Script::new(
        "dangerous".to_string(),
        "rm -rf /".to_string(),
        ScriptLanguage::Bash,
    );
    assert!(!script.is_safe());
}

#[test]
fn test_script_is_not_safe_fork_bomb() {
    let script = Script::new(
        "bomb".to_string(),
        ":(){ :|:& };:".to_string(),
        ScriptLanguage::Bash,
    );
    assert!(!script.is_safe());
}

#[test]
fn test_script_is_not_safe_mkfs() {
    let script = Script::new(
        "wipe".to_string(),
        "mkfs.ext4 /dev/sda".to_string(),
        ScriptLanguage::Bash,
    );
    assert!(!script.is_safe());
}

#[test]
fn test_success_rate_no_runs() {
    let script = Script::new("t".to_string(), "echo t".to_string(), ScriptLanguage::Bash);
    assert_eq!(script.success_rate(), 0.0);
}

#[test]
fn test_success_rate_all_success() {
    let mut script = Script::new("t".to_string(), "echo t".to_string(), ScriptLanguage::Bash);
    script.metadata.success_count = 10;
    script.metadata.failure_count = 0;
    assert_eq!(script.success_rate(), 100.0);
}

#[test]
fn test_success_rate_mixed() {
    let mut script = Script::new("t".to_string(), "echo t".to_string(), ScriptLanguage::Bash);
    script.metadata.success_count = 3;
    script.metadata.failure_count = 1;
    assert_eq!(script.success_rate(), 75.0);
}

#[test]
fn test_execution_record_successful() {
    let record = ExecutionRecord {
        id: "r1".to_string(),
        script_id: "s1".to_string(),
        script_version: "v1.0.0".to_string(),
        executed_by: "user".to_string(),
        executed_at: Utc::now(),
        exit_code: 0,
        duration_ms: 500,
        output: Some("ok".to_string()),
        error: None,
        context: ScriptContext {
            directory: None,
            git_repo: None,
            git_branch: None,
            environment: HashMap::new(),
        },
    };
    assert!(record.was_successful());
}

#[test]
fn test_execution_record_failed() {
    let record = ExecutionRecord {
        id: "r2".to_string(),
        script_id: "s1".to_string(),
        script_version: "v1.0.0".to_string(),
        executed_by: "user".to_string(),
        executed_at: Utc::now(),
        exit_code: 1,
        duration_ms: 200,
        output: None,
        error: Some("error".to_string()),
        context: ScriptContext {
            directory: None,
            git_repo: None,
            git_branch: None,
            environment: HashMap::new(),
        },
    };
    assert!(!record.was_successful());
}

#[test]
fn test_language_extension_round_trip() {
    let cases = vec![
        (ScriptLanguage::Bash, "sh"),
        (ScriptLanguage::Shell, "sh"),
        (ScriptLanguage::Python, "py"),
        (ScriptLanguage::JavaScript, "js"),
        (ScriptLanguage::Ruby, "rb"),
        (ScriptLanguage::Perl, "pl"),
        (ScriptLanguage::PowerShell, "ps1"),
        (ScriptLanguage::Batch, "bat"),
    ];
    for (lang, ext) in cases {
        assert_eq!(lang.extension(), ext);
    }
}

#[test]
fn test_language_from_extension_all_supported() {
    assert_eq!(ScriptLanguage::from_extension("sh"), ScriptLanguage::Shell);
    assert_eq!(ScriptLanguage::from_extension("bash"), ScriptLanguage::Bash);
    assert_eq!(ScriptLanguage::from_extension("py"), ScriptLanguage::Python);
    assert_eq!(ScriptLanguage::from_extension("js"), ScriptLanguage::JavaScript);
    assert_eq!(ScriptLanguage::from_extension("rb"), ScriptLanguage::Ruby);
    assert_eq!(ScriptLanguage::from_extension("pl"), ScriptLanguage::Perl);
    assert_eq!(ScriptLanguage::from_extension("ps1"), ScriptLanguage::PowerShell);
    assert_eq!(ScriptLanguage::from_extension("bat"), ScriptLanguage::Batch);
    assert_eq!(ScriptLanguage::from_extension("cmd"), ScriptLanguage::Batch);
    assert_eq!(ScriptLanguage::from_extension("xyz"), ScriptLanguage::Unknown);
}

#[test]
fn test_normalize_git_url_https_with_extension() {
    assert_eq!(
        normalize_git_url("https://github.com/user/repo.git"),
        "github.com/user/repo"
    );
}

#[test]
fn test_normalize_git_url_ssh() {
    assert_eq!(
        normalize_git_url("git@github.com:user/repo.git"),
        "github.com/user/repo"
    );
}

#[test]
fn test_normalize_git_url_already_clean() {
    assert_eq!(
        normalize_git_url("https://github.com/user/repo"),
        "github.com/user/repo"
    );
}

#[test]
fn test_contexts_match_by_git_repo() {
    let ctx1 = ScriptContext {
        directory: Some("/home/user/a".to_string()),
        git_repo: Some("github.com/user/repo".to_string()),
        git_branch: Some("main".to_string()),
        environment: HashMap::new(),
    };
    let ctx2 = ScriptContext {
        directory: Some("/home/user/b".to_string()),
        git_repo: Some("github.com/user/repo".to_string()),
        git_branch: Some("develop".to_string()),
        environment: HashMap::new(),
    };
    assert!(contexts_match(&ctx1, &ctx2));
}

#[test]
fn test_contexts_match_by_exact_directory() {
    let ctx = ScriptContext {
        directory: Some("/home/user/project".to_string()),
        git_repo: None,
        git_branch: None,
        environment: HashMap::new(),
    };
    assert!(contexts_match(&ctx, &ctx.clone()));
}

#[test]
fn test_contexts_match_by_parent_directory() {
    let parent = ScriptContext {
        directory: Some("/home/user/project".to_string()),
        git_repo: None,
        git_branch: None,
        environment: HashMap::new(),
    };
    let child = ScriptContext {
        directory: Some("/home/user/project/src".to_string()),
        git_repo: None,
        git_branch: None,
        environment: HashMap::new(),
    };
    assert!(contexts_match(&parent, &child));
    assert!(contexts_match(&child, &parent));
}

#[test]
fn test_contexts_do_not_match_different_repos() {
    let ctx1 = ScriptContext {
        directory: Some("/home/user/a".to_string()),
        git_repo: Some("github.com/user/repo1".to_string()),
        git_branch: None,
        environment: HashMap::new(),
    };
    let ctx2 = ScriptContext {
        directory: Some("/home/user/b".to_string()),
        git_repo: Some("github.com/user/repo2".to_string()),
        git_branch: None,
        environment: HashMap::new(),
    };
    assert!(!contexts_match(&ctx1, &ctx2));
}

#[test]
fn test_large_script_saved_and_loaded_intact() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    let mut content = String::from("#!/bin/bash\n");
    for i in 0..500 {
        content.push_str(&format!("echo 'line {}'\n", i));
    }
    let script = make_script("large", &content);
    s.save_script(&script).unwrap();
    let loaded = s.load_script_by_name("large").unwrap();
    assert_eq!(loaded.content, content);
    assert_eq!(loaded.metadata.line_count, content.lines().count());
}

#[test]
fn test_special_characters_in_content_preserved() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    let content = "#!/bin/bash\necho \"!@#$%^&*()\"\necho 'quotes and spaces'\n";
    let script = make_script("special", content);
    s.save_script(&script).unwrap();
    let loaded = s.load_script_by_name("special").unwrap();
    assert_eq!(loaded.content, content);
}

#[test]
fn test_empty_vault_list_returns_empty() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    assert!(s.list_scripts().unwrap().is_empty());
}

#[test]
fn test_script_not_found_by_name_errors() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    assert!(s.load_script_by_name("nonexistent").is_err());
}

#[test]
fn test_script_not_found_by_id_errors() {
    let tmp = TempDir::new().unwrap();
    let s = storage(&tmp);
    assert!(s.load_script("00000000-0000-0000-0000-000000000000").is_err());
}

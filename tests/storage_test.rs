use serde_json::json;
use wasm_kv_db::Storage;

#[test]
fn test_storage_put_get_delete() {
    let storage = Storage::new();

    // 1. Put a value
    let key = "user:123";
    let value = json!({"name": "Alice", "role": "engineer"});
    let inserted = storage.put(key, value.clone()).unwrap();
    assert_eq!(inserted, value);

    // 2. Get the value
    let retrieved = storage.get_raw(key).unwrap();
    assert_eq!(retrieved, value);

    // 3. Check existence
    assert!(storage.exists(key));

    // 4. List keys
    let keys = storage.list_keys();
    assert_eq!(keys, vec!["user:123".to_string()]);

    // 5. Delete the value
    let deleted = storage.delete(key).unwrap();
    assert_eq!(deleted, value);

    // 6. Verify it's gone
    assert!(!storage.exists(key));
    assert!(storage.get_raw(key).is_err());
}

#[test]
fn test_storage_put_update() {
    let storage = Storage::new();
    let key = "user:456";

    // Initial insert
    let initial = json!({"name": "Bob", "score": 10});
    storage.put(key, initial.clone()).unwrap();
    assert_eq!(storage.get_raw(key).unwrap(), initial);

    // Update
    let updated = json!({"name": "Bob", "score": 20});
    storage.put(key, updated.clone()).unwrap();
    assert_eq!(storage.get_raw(key).unwrap(), updated);
}

#[test]
fn test_storage_get_nonexistent() {
    let storage = Storage::new();
    let result = storage.get_raw("nonexistent");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, wasm_kv_db::AppError::KeyNotFound(_)));
}

#[test]
fn test_storage_delete_nonexistent() {
    let storage = Storage::new();
    let result = storage.delete("nonexistent");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, wasm_kv_db::AppError::KeyNotFound(_)));
}

#[test]
fn test_storage_multiple_keys() {
    let storage = Storage::new();

    let entries = vec![
        ("key1", json!({"a": 1})),
        ("key2", json!({"b": 2})),
        ("key3", json!({"c": 3})),
    ];

    for (key, value) in &entries {
        storage.put(key, value.clone()).unwrap();
    }

    let keys = storage.list_keys();
    assert_eq!(keys.len(), 3);
    assert!(keys.contains(&"key1".to_string()));
    assert!(keys.contains(&"key2".to_string()));
    assert!(keys.contains(&"key3".to_string()));

    // Verify each value
    for (key, expected) in &entries {
        let actual = storage.get_raw(key).unwrap();
        assert_eq!(&actual, expected);
    }
}

use wasm_kv_db::{AppError, Storage};

#[tokio::test]
pub async fn test_persistence() -> Result<(), AppError> {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("db");

    let storage = Storage::new_with_persistence(&db_path)?;
    storage.put("key1", b"value1".to_vec())?;
    storage.put("key2", b"value2".to_vec())?;
    storage.sync()?;
    drop(storage);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let storage = Storage::new_with_persistence(&db_path)?;
    assert_eq!(storage.get("key1")?, b"value1");
    assert_eq!(storage.get("key2")?, b"value2");

    Ok(())
}

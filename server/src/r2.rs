use anyhow::{Result, anyhow};
use aws_credential_types::Credentials;
use aws_sdk_s3::{
    Client,
    config::{BehaviorVersion, Builder, Region},
    operation::get_object::GetObjectError,
    primitives::ByteStream,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub struct R2Client {
    client: Client,
    bucket: String,
}

impl R2Client {
    pub fn new(
        account_id: &str,
        access_key_id: &str,
        secret_access_key: &str,
        bucket: &str,
    ) -> Self {
        let credentials =
            Credentials::new(access_key_id, secret_access_key, None, None, "scriptvault");
        let config = Builder::new()
            .behavior_version(BehaviorVersion::latest())
            .endpoint_url(format!("https://{}.r2.cloudflarestorage.com", account_id))
            .region(Region::new("auto"))
            .credentials_provider(credentials)
            .force_path_style(false)
            .build();
        Self {
            client: Client::from_conf(config),
            bucket: bucket.to_string(),
        }
    }

    fn script_key(user_id: &str, script_id: &str) -> String {
        format!("users/{}/scripts/{}.json", user_id, script_id)
    }

    fn meta_key(user_id: &str) -> String {
        format!("users/{}/index.json", user_id)
    }

    fn compute_etag(bytes: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(bytes);
        hex::encode(h.finalize())
    }

    pub async fn list_script_metas(&self, user_id: &str) -> Result<Vec<Value>> {
        let key = Self::meta_key(user_id);
        match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(out) => {
                let bytes = out.body.collect().await?.into_bytes();
                Ok(serde_json::from_slice::<Vec<Value>>(&bytes).unwrap_or_default())
            }
            Err(e) if matches!(e.as_service_error(), Some(GetObjectError::NoSuchKey(_))) => {
                Ok(vec![])
            }
            Err(e) => Err(anyhow!("failed to read script index: {}", e)),
        }
    }

    pub async fn get_script(&self, user_id: &str, script_id: &str) -> Result<(Value, String)> {
        let key = Self::script_key(user_id, script_id);
        match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(out) => {
                let bytes = out.body.collect().await?.into_bytes();
                let etag = Self::compute_etag(&bytes);
                let value = serde_json::from_slice(&bytes)?;
                Ok((value, etag))
            }
            Err(e) if matches!(e.as_service_error(), Some(GetObjectError::NoSuchKey(_))) => {
                Err(anyhow!("script not found: {}", script_id))
            }
            Err(e) => Err(anyhow!("failed to read script: {}", e)),
        }
    }

    pub async fn put_script(
        &self,
        user_id: &str,
        script_id: &str,
        content: &Value,
        if_match: Option<&str>,
    ) -> Result<String> {
        let key = Self::script_key(user_id, script_id);

        if let Some(expected_etag) = if_match {
            match self
                .client
                .get_object()
                .bucket(&self.bucket)
                .key(&key)
                .send()
                .await
            {
                Ok(out) => {
                    let bytes = out.body.collect().await?.into_bytes();
                    let current_etag = Self::compute_etag(&bytes);
                    if current_etag != expected_etag {
                        return Err(anyhow!("etag_mismatch"));
                    }
                }
                Err(e) if matches!(e.as_service_error(), Some(GetObjectError::NoSuchKey(_))) => {
                    return Err(anyhow!("etag_mismatch"));
                }
                Err(e) => return Err(anyhow!("failed to check etag: {}", e)),
            }
        }

        let bytes = serde_json::to_vec(content)?;
        let etag = Self::compute_etag(&bytes);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .content_type("application/json")
            .body(ByteStream::from(bytes))
            .send()
            .await
            .map_err(|e| anyhow!("failed to write script: {}", e))?;

        self.update_index(user_id, script_id, content).await?;

        Ok(etag)
    }

    pub async fn delete_script(&self, user_id: &str, script_id: &str) -> Result<()> {
        let key = Self::script_key(user_id, script_id);
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| anyhow!("failed to delete script: {}", e))?;
        self.remove_from_index(user_id, script_id).await
    }

    async fn update_index(&self, user_id: &str, script_id: &str, script: &Value) -> Result<()> {
        let mut metas = self.list_script_metas(user_id).await?;
        metas.retain(|m| m.get("id").and_then(|v| v.as_str()) != Some(script_id));
        if let Some(meta) = build_meta(script_id, script) {
            metas.push(meta);
        }
        self.write_index(user_id, &metas).await
    }

    async fn remove_from_index(&self, user_id: &str, script_id: &str) -> Result<()> {
        let mut metas = self.list_script_metas(user_id).await?;
        metas.retain(|m| m.get("id").and_then(|v| v.as_str()) != Some(script_id));
        self.write_index(user_id, &metas).await
    }

    async fn write_index(&self, user_id: &str, metas: &[Value]) -> Result<()> {
        let key = Self::meta_key(user_id);
        let bytes = serde_json::to_vec(metas)?;
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .content_type("application/json")
            .body(ByteStream::from(bytes))
            .send()
            .await
            .map_err(|e| anyhow!("failed to write index: {}", e))?;
        Ok(())
    }
}

fn build_meta(script_id: &str, script: &Value) -> Option<Value> {
    Some(serde_json::json!({
        "id": script_id,
        "name": script.get("name")?.as_str()?,
        "version": script.get("version")?.as_str()?,
        "updated_at": script.get("updated_at")?.as_str()?,
        "hash": script.get("metadata")?.get("hash")?.as_str()?,
    }))
}

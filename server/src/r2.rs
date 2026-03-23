use anyhow::{Result, anyhow};
use aws_credential_types::Credentials;
use aws_sdk_s3::{
    Client,
    config::{BehaviorVersion, Builder, Region},
    operation::get_object::GetObjectError,
    primitives::ByteStream,
};
use serde_json::Value;

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

    fn vault_key(user_id: &str) -> String {
        format!("users/{}/vault.json", user_id)
    }

    pub async fn get_vault(&self, user_id: &str) -> Result<Value> {
        let key = Self::vault_key(user_id);

        match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(output) => {
                let bytes = output.body.collect().await?.into_bytes();
                let value: Value = serde_json::from_slice(&bytes)?;
                Ok(value)
            }
            Err(e) => {
                if matches!(e.as_service_error(), Some(GetObjectError::NoSuchKey(_))) {
                    Ok(Value::Array(vec![]))
                } else {
                    Err(anyhow!("Failed to read vault from R2: {}", e))
                }
            }
        }
    }

    pub async fn put_vault(&self, user_id: &str, content: &Value) -> Result<()> {
        let key = Self::vault_key(user_id);
        let bytes = serde_json::to_vec(content)?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .content_type("application/json")
            .body(ByteStream::from(bytes))
            .send()
            .await
            .map_err(|e| anyhow!("Failed to write vault to R2: {}", e))?;

        Ok(())
    }
}

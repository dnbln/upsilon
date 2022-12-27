/*
 *        Copyright (c) 2022 Dinu Blanovschi
 *
 *    Licensed under the Apache License, Version 2.0 (the "License");
 *    you may not use this file except in compliance with the License.
 *    You may obtain a copy of the License at
 *
 *        https://www.apache.org/licenses/LICENSE-2.0
 *
 *    Unless required by applicable law or agreed to in writing, software
 *    distributed under the License is distributed on an "AS IS" BASIS,
 *    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *    See the License for the specific language governing permissions and
 *    limitations under the License.
 */

use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;

use crate::TestResult;

struct ClientCore {
    root: String,
    gql: String,
    inner: reqwest::Client,
}

#[derive(Clone)]
pub struct Client {
    core: Arc<ClientCore>,
    token: Option<String>,
}

impl Client {
    pub fn new(root: impl Into<String>) -> Self {
        let root = root.into();
        Self {
            core: Arc::new(ClientCore {
                gql: format!("{root}/graphql"),
                root,
                inner: reqwest::Client::new(),
            }),
            token: None,
        }
    }

    pub fn with_token(&self, token: impl Into<String>) -> Self {
        Self {
            core: Arc::clone(&self.core),
            token: Some(token.into()),
        }
    }

    async fn _gql_query<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: HashMap<String, serde_json::Value>,
    ) -> TestResult<T> {
        let mut req = self.core.inner.post(&self.core.gql);
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }

        #[derive(Deserialize)]
        struct GqlResponse<T> {
            data: T,
        }

        let res = req
            .json(&serde_json::json!({
                "query": query,
                "variables": variables,
            }))
            .send()
            .await?;

        #[cfg(debug_assertions)]
        let data = {
            let t = res.text().await?;

            println!("GQL response: {t}");

            serde_json::from_str::<GqlResponse<T>>(&t)?.data
        };

        #[cfg(not(debug_assertions))]
        let data = res.json::<GqlResponse<T>>().await?.data;

        Ok(data)
    }

    pub async fn gql_query<T: for<'de> Deserialize<'de>>(&self, query: &str) -> TestResult<T> {
        self.gql_query_with_variables(query, HashMap::new()).await
    }

    pub async fn gql_query_with_variables<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: HashMap<String, serde_json::Value>,
    ) -> TestResult<T> {
        self._gql_query(query, variables).await
    }

    pub async fn post_empty(&self, path: &str) -> TestResult<()> {
        self.core
            .inner
            .post(format!("{}{path}", self.core.root))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(from = "serde_json::Value")]
pub struct Anything;

impl From<serde_json::Value> for Anything {
    fn from(_value: serde_json::Value) -> Self {
        Self
    }
}

#[macro_export]
macro_rules! gql_vars {
    ($($name:literal: $value:tt),* $(,)?) => {
        std::collections::HashMap::from([
            $(
                ($name.to_string(), $crate::serde_json::json!($value)),
            )*
        ])
    };
}

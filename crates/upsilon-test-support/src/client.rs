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

use serde::Deserialize;

pub struct Client {
    root: String,
    gql: String,
    inner: reqwest::Client,
    token: Option<String>,
}

impl Client {
    pub fn new(root: impl Into<String>) -> Self {
        let root = root.into();
        Self {
            gql: format!("{}/graphql", root),
            root,
            inner: reqwest::Client::new(),
            token: None,
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn set_token(&mut self, token: impl Into<String>) {
        self.token = Some(token.into());
    }

    async fn _gql_query<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: HashMap<String, serde_json::Value>,
    ) -> Result<T, reqwest::Error> {
        let mut req = self.inner.post(&self.gql);
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }

        #[derive(Deserialize)]
        struct GqlResponse<T> {
            data: T,
        }

        let data = req
            .json(&serde_json::json!({
                "query": query,
                "variables": variables,
            }))
            .send()
            .await?
            .json::<GqlResponse<T>>()
            .await?
            .data;

        Ok(data)
    }

    pub async fn gql_query<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
    ) -> Result<T, reqwest::Error> {
        self.gql_query_with_variables(query, HashMap::new()).await
    }

    pub async fn gql_query_with_variables<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: HashMap<String, serde_json::Value>,
    ) -> Result<T, reqwest::Error> {
        self._gql_query(query, variables).await
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

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
use serde_json::Value;

use crate::DDDResult;

pub(crate) struct Client {
    reqwest_client: reqwest::Client,
    root: String,
    root_graphql: String,
    token: Option<String>,
}

impl Client {
    pub(crate) fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    pub(crate) fn new(port: u16) -> Self {
        let reqwest_client = reqwest::Client::new();
        let root = format!("http://localhost:{}", port);
        let root_graphql = format!("{}/graphql", root);

        Self {
            reqwest_client,
            root,
            root_graphql,
            token: None,
        }
    }

    async fn _graphql_query<D: for<'d> Deserialize<'d> + 'static>(
        &self,
        q: &str,
        variables: HashMap<&str, Value>,
    ) -> DDDResult<D> {
        #[derive(Deserialize)]
        struct GraphQLResponse<D>
        where
            D: 'static,
        {
            data: D,
        }

        let mut req = self
            .reqwest_client
            .post(&self.root_graphql)
            .json(&serde_json::json!({
                "query": q,
                "variables": variables,
            }));

        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }

        #[cfg(not(feature = "dump_gql_response"))]
        let res = req.send().await?.json::<GraphQLResponse<D>>().await?;

        #[cfg(feature = "dump_gql_response")]
        let res = {
            let txt = req.send().await?.text().await?;

            log::info!("response: {}", txt);

            serde_json::from_str::<GraphQLResponse<D>>(&txt)?
        };

        Ok(res.data)
    }

    pub(crate) async fn gql_query_with_variables<D: for<'d> Deserialize<'d> + 'static>(
        &self,
        query: &str,
        variables: HashMap<&str, Value>,
    ) -> DDDResult<D> {
        self._graphql_query(query, variables).await
    }

    pub(crate) async fn gql_mutation_with_variables<D: for<'d> Deserialize<'d> + 'static>(
        &self,
        mutation: &str,
        variables: HashMap<&str, Value>,
    ) -> DDDResult<D> {
        self._graphql_query(mutation, variables).await
    }

    pub(crate) async fn gql_query<D: for<'d> Deserialize<'d> + 'static>(
        &self,
        query: &str,
    ) -> DDDResult<D> {
        self.gql_query_with_variables(query, HashMap::new()).await
    }

    pub(crate) async fn gql_mutation<D: for<'d> Deserialize<'d> + 'static>(
        &self,
        mutation: &str,
    ) -> DDDResult<D> {
        self.gql_mutation_with_variables(mutation, HashMap::new())
            .await
    }
}

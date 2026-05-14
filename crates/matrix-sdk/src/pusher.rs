// Copyright 2024 The Matrix.org Foundation C.I.C.
// Copyright 2024 Hanadi Tamimi
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! High-level pusher API.

use ruma::api::client::push::{PusherIds, set_pusher, set_pusher::v3::PusherAction};

use crate::{Client, Result};

/// Parameters for [`Pusher::set`].
///
/// `append` controls the Matrix spec's `append` flag on
/// `POST /_matrix/client/v3/pushers/set`: when `true`, the homeserver keeps
/// any existing pusher with the same `app_id` and `pushkey` for other users
/// alongside the new one; when `false` (the default), such pushers are
/// replaced.
#[derive(Debug, Clone)]
pub struct SetPusherRequest {
    /// The pusher to register.
    pub pusher: ruma::api::client::push::Pusher,
    /// Whether to keep existing pushers with the same `app_id` and `pushkey`.
    pub append: bool,
}

impl From<ruma::api::client::push::Pusher> for SetPusherRequest {
    fn from(pusher: ruma::api::client::push::Pusher) -> Self {
        Self { pusher, append: false }
    }
}

/// A high-level API to interact with the pusher API.
///
/// All the methods in this struct send a request to the homeserver.
#[derive(Debug, Clone)]
pub struct Pusher {
    /// The underlying HTTP client.
    client: Client,
}

impl Pusher {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Sets a given pusher.
    pub async fn set(&self, request: impl Into<SetPusherRequest>) -> Result<()> {
        let SetPusherRequest { pusher, append } = request.into();
        let mut request = set_pusher::v3::Request::post(pusher);
        if let PusherAction::Post(data) = &mut request.action {
            data.append = append;
        }
        self.client.send(request).await?;
        Ok(())
    }

    /// Deletes a pusher by its ids
    pub async fn delete(&self, pusher_ids: PusherIds) -> Result<()> {
        let request = set_pusher::v3::Request::delete(pusher_ids);
        self.client.send(request).await?;
        Ok(())
    }
}

// The http mocking library is not supported for wasm32
#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use matrix_sdk_test::{async_test, test_json};
    use ruma::{
        api::client::push::{PusherIds, PusherInit, PusherKind},
        push::HttpPusherData,
    };
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_partial_json, method, path},
    };

    use super::SetPusherRequest;
    use crate::test_utils::logged_in_client;

    async fn mock_api(server: MockServer) {
        Mock::given(method("POST"))
            .and(path("_matrix/client/r0/pushers/set"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&*test_json::EMPTY))
            .mount(&server)
            .await;
    }

    fn dummy_pusher() -> PusherInit {
        PusherInit {
            ids: PusherIds::new("pushKey".to_owned(), "app_id".to_owned()),
            app_display_name: "name".to_owned(),
            kind: PusherKind::Http(HttpPusherData::new("dummy".to_owned())),
            lang: "EN".to_owned(),
            device_display_name: "name".to_owned(),
            profile_tag: None,
        }
    }

    #[async_test]
    async fn test_set_pusher() {
        let server = MockServer::start().await;
        let client = logged_in_client(Some(server.uri())).await;
        mock_api(server).await;

        let pusher: ruma::api::client::push::Pusher = dummy_pusher().into();
        let response = client.pusher().set(pusher).await;

        assert!(response.is_ok());
    }

    #[async_test]
    async fn test_set_pusher_with_append() {
        let server = MockServer::start().await;
        let client = logged_in_client(Some(server.uri())).await;

        Mock::given(method("POST"))
            .and(path("_matrix/client/r0/pushers/set"))
            .and(body_partial_json(json!({ "append": true })))
            .respond_with(ResponseTemplate::new(200).set_body_json(&*test_json::EMPTY))
            .mount(&server)
            .await;

        let response = client
            .pusher()
            .set(SetPusherRequest { pusher: dummy_pusher().into(), append: true })
            .await;

        assert!(response.is_ok());
    }

    #[async_test]
    async fn test_delete_pusher() {
        let server = MockServer::start().await;
        let client = logged_in_client(Some(server.uri())).await;
        mock_api(server).await;

        // prepare pusher ids
        let pusher_ids = PusherIds::new("pushKey".to_owned(), "app_id".to_owned());

        let response = client.pusher().delete(pusher_ids).await;

        assert!(response.is_ok());
    }
}

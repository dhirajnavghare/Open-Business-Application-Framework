// Copyright 2026 Dhiraj Navghare
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

use axum::{
    http::header,
    response::{Html, IntoResponse, Response},
};

const ADMIN_UI_HTML: &str = include_str!("admin_ui.html");

/// Serve the embedded self-contained Super Admin Single Page Application.
pub async fn serve_admin_ui() -> Response {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        Html(ADMIN_UI_HTML),
    )
        .into_response()
}

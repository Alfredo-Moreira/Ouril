use axum::{extract::State, Json};
use ouril_protocol::{ApiVersions, Meta, PlatformVersions, RealtimeProto, API_VERSION};

use crate::{config::Platforms, state::AppState};

fn platforms(p: &Platforms) -> PlatformVersions {
    PlatformVersions {
        ios: p.ios.clone(),
        android: p.android.clone(),
        web: p.web.clone(),
    }
}

/// `GET /v1/meta` (public).
pub async fn meta(State(state): State<AppState>) -> Json<Meta> {
    let c = &state.config;
    Json(Meta {
        min_supported: platforms(&c.min_supported),
        recommended: platforms(&c.recommended),
        api: ApiVersions {
            current: API_VERSION.to_string(),
            deprecated: vec![],
        },
        realtime_proto: RealtimeProto {
            current: ouril_protocol::REALTIME_PROTO_CURRENT,
            min: ouril_protocol::REALTIME_PROTO_MIN,
        },
    })
}

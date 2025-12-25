//! UDP Socket options and extra data

use std::sync::Arc;

use crate::config::ServerUser;

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct UdpSocketControlData {
    /// Session ID in client.
    ///
    /// For identifying an unique association in client
    pub client_session_id: u64,
    /// Session ID in server.
    ///
    /// For identifying an unique association in server
    pub server_session_id: u64,
    /// Packet counter
    pub packet_id: u64,
    /// Server user instance
    pub user: Option<Arc<ServerUser>>,
    /// Timestamp diff for ComplyWithIncoming (local now - incoming timestamp)
    pub timestamp_diff: i64,
}

impl UdpSocketControlData {
    pub fn without_timestamp_diff(self) -> Self {
        Self {
            client_session_id: self.client_session_id,
            server_session_id: self.server_session_id,
            packet_id: self.packet_id,
            user: self.user,
            timestamp_diff: 0,
        }
    }
}

use nara_protocol::VoiceState;

#[derive(Debug, Clone)]
pub struct VoiceSession {
    state: VoiceState,
}

impl Default for VoiceSession {
    fn default() -> Self {
        Self {
            state: VoiceState::Idle,
        }
    }
}

impl VoiceSession {
    pub fn state(&self) -> &VoiceState {
        &self.state
    }

    pub fn set_state(&mut self, state: VoiceState) {
        self.state = state;
    }
}

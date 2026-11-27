//! Remembers Vietnamese or English per application.

use crate::state::AppState;

#[derive(Debug, Default)]
pub struct SmartSwitch {
    current: Option<String>,
}

impl SmartSwitch {
    #[must_use]
    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    /// Records that `app` is now in front. Stores the mode of the previous
    /// application and returns the remembered mode of `app`, if any.
    pub fn app_changed(
        &mut self,
        app: &str,
        vietnamese: bool,
        state: Option<&mut AppState>,
    ) -> Option<bool> {
        if self.current.as_deref() == Some(app) {
            return None;
        }
        let previous = self.current.replace(app.to_owned());
        let state = state?;
        if let Some(previous) = previous {
            state.apps.insert(previous, vietnamese);
        }
        state.apps.get(app).copied()
    }

    /// Stores `vietnamese` for the application in front.
    pub fn remember(&self, vietnamese: bool, state: &mut AppState) {
        if let Some(app) = &self.current {
            state.apps.insert(app.clone(), vietnamese);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_mode_per_app() {
        let mut state = AppState::default();
        let mut switch = SmartSwitch::default();
        assert_eq!(switch.app_changed("editor", true, Some(&mut state)), None);
        assert_eq!(switch.app_changed("editor", true, Some(&mut state)), None);
        assert_eq!(switch.app_changed("terminal", true, Some(&mut state)), None);
        assert_eq!(state.apps.get("editor"), Some(&true));
        switch.remember(false, &mut state);
        assert_eq!(
            switch.app_changed("editor", false, Some(&mut state)),
            Some(true)
        );
        assert_eq!(
            switch.app_changed("terminal", true, Some(&mut state)),
            Some(false)
        );
    }
}

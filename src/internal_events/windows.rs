use metrics::counter;
use vector_core::internal_event::InternalEvent;

#[derive(Debug)]
pub(crate) struct WindowsServiceStart<'a> {
    pub(crate) already_started: bool,
    pub(crate) name: &'a str,
}

impl<'a> InternalEvent for WindowsServiceStart<'a> {
    fn emit_logs(&self) {
        info!(
            already_started = %self.already_started,
            name = self.name,
            "Started Windows Service.",
        );
    }

    fn emit_metrics(&self) {
        counter!("windows_service_start_total", 1,
            "already_started" => self.already_started.to_string(),
        );
    }
}

#[derive(Debug)]
pub(crate) struct WindowsServiceStop<'a> {
    pub(crate) already_stopped: bool,
    pub(crate) name: &'a str,
}

impl<'a> InternalEvent for WindowsServiceStop<'a> {
    fn emit_logs(&self) {
        info!(
            already_stopped = %self.already_stopped,
            name = ?self.name,
            "Stopped Windows Service.",
        );
    }

    fn emit_metrics(&self) {
        counter!("windows_service_stop_total", 1,
            "already_stopped" => self.already_stopped.to_string(),
        );
    }
}

#[derive(Debug)]
pub(crate) struct WindowsServiceRestart<'a> {
    pub(crate) name: &'a str,
}

impl<'a> InternalEvent for WindowsServiceRestart<'a> {
    fn emit_logs(&self) {
        info!(
            name = ?self.name,
            "Restarted Windows Service."
        );
    }

    fn emit_metrics(&self) {
        counter!("windows_service_restart_total", 1)
    }
}

#[derive(Debug)]
pub(crate) struct WindowsServiceInstall<'a> {
    pub(crate) name: &'a str,
}

impl<'a> InternalEvent for WindowsServiceInstall<'a> {
    fn emit_logs(&self) {
        info!(
            name = ?self.name,
            "Installed Windows Service.",
        );
    }

    fn emit_metrics(&self) {
        counter!("windows_service_install_total", 1,);
    }
}

#[derive(Debug)]
pub(crate) struct WindowsServiceUninstall<'a> {
    pub(crate) name: &'a str,
}

impl<'a> InternalEvent for WindowsServiceUninstall<'a> {
    fn emit_logs(&self) {
        info!(
            name = ?self.name,
            "Uninstalled Windows Service.",
        );
    }

    fn emit_metrics(&self) {
        counter!("windows_service_uninstall_total", 1,);
    }
}

#[derive(Debug)]
pub(crate) struct WindowsServiceDoesNotExist<'a> {
    pub(crate) name: &'a str,
}

impl<'a> InternalEvent for WindowsServiceDoesNotExist<'a> {
    fn emit_logs(&self) {
        error!(
            name = self.name,
            "Windows service does not exist. Maybe it needs to be installed?",
        );
    }

    fn emit_metrics(&self) {
        counter!("windows_service_does_not_exist_total", 1,);
    }
}

use std::path::Path;
use windows::core::{Interface, BSTR, VARIANT};
use windows::Win32::Foundation::VARIANT_BOOL;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::TaskScheduler::{
    IExecAction, ILogonTrigger, ISessionStateChangeTrigger, ITaskFolder, ITaskService,
    TaskScheduler, TASK_ACTION_EXEC, TASK_CREATE_OR_UPDATE, TASK_INSTANCES_IGNORE_NEW,
    TASK_LOGON_INTERACTIVE_TOKEN, TASK_SESSION_UNLOCK, TASK_TRIGGER_LOGON,
    TASK_TRIGGER_SESSION_STATE_CHANGE,
};

pub const STARTUP_TASK_NAME: &str = "puwvga_ApplyColor";

fn current_user_id() -> Option<String> {
    let user = std::env::var("USERNAME").ok()?;
    let user = user.trim();
    if user.is_empty() {
        return None;
    }
    match std::env::var("USERDOMAIN") {
        Ok(domain) if !domain.trim().is_empty() => Some(format!("{}\\{}", domain.trim(), user)),
        _ => Some(user.to_string()),
    }
}

fn with_task_root<T>(f: impl FnOnce(&ITaskService, &ITaskFolder) -> Result<T, String>) -> Result<T, String> {
    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let needs_uninit = hr.is_ok();
        let res = (|| {
            let service: ITaskService =
                CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER)
                    .map_err(|e| format!("CoCreateInstance(TaskScheduler): {e}"))?;
            service
                .Connect(
                    &VARIANT::default(),
                    &VARIANT::default(),
                    &VARIANT::default(),
                    &VARIANT::default(),
                )
                .map_err(|e| format!("ITaskService::Connect: {e}"))?;
            let root = service
                .GetFolder(&BSTR::from(r"\"))
                .map_err(|e| format!("ITaskService::GetFolder: {e}"))?;
            f(&service, &root)
        })();
        if needs_uninit {
            CoUninitialize();
        }
        res
    }
}

/// Returns true if the Windows Task Scheduler task `puwvga_ApplyColor` exists and is enabled.
pub fn is_apply_on_startup_enabled() -> bool {
    with_task_root(|_, root| unsafe {
        let task = root
            .GetTask(&BSTR::from(STARTUP_TASK_NAME))
            .map_err(|e| e.to_string())?;
        let enabled = task.Enabled().map_err(|e| e.to_string())?;
        Ok(enabled.as_bool())
    })
    .unwrap_or(false)
}

/// Registers or deletes the Windows Task Scheduler task `puwvga_ApplyColor`.
/// Triggers on user logon (2s delay) and workstation unlock (1s delay), running
/// `puwvga.exe --apply-only` without admin rights and exiting immediately.
pub fn set_apply_on_startup(enabled: bool, exe_path: &Path) -> Result<(), String> {
    with_task_root(|service, root| unsafe {
        if !enabled {
            let _ = root.DeleteTask(&BSTR::from(STARTUP_TASK_NAME), 0);
            return Ok(());
        }

        let user_id = current_user_id()
            .ok_or_else(|| "Cannot determine current Windows USERNAME".to_string())?;
        let user_bstr = BSTR::from(user_id.as_str());

        let task_def = service
            .NewTask(0)
            .map_err(|e| format!("ITaskService::NewTask: {e}"))?;

        if let Ok(principal) = task_def.Principal() {
            let _ = principal.SetUserId(&user_bstr);
            let _ = principal.SetLogonType(TASK_LOGON_INTERACTIVE_TOKEN);
        }

        if let Ok(reg_info) = task_def.RegistrationInfo() {
            let _ = reg_info.SetAuthor(&user_bstr);
            let _ = reg_info.SetDescription(&BSTR::from(
                "Re-apply puwvga display color settings on Windows logon and session unlock.",
            ));
        }

        if let Ok(settings) = task_def.Settings() {
            let _ = settings.SetEnabled(VARIANT_BOOL::from(true));
            let _ = settings.SetStartWhenAvailable(VARIANT_BOOL::from(true));
            let _ = settings.SetAllowDemandStart(VARIANT_BOOL::from(true));
            let _ = settings.SetDisallowStartIfOnBatteries(VARIANT_BOOL::from(false));
            let _ = settings.SetStopIfGoingOnBatteries(VARIANT_BOOL::from(false));
            let _ = settings.SetMultipleInstances(TASK_INSTANCES_IGNORE_NEW);
            let _ = settings.SetExecutionTimeLimit(&BSTR::from("PT1M"));
        }

        let triggers = task_def
            .Triggers()
            .map_err(|e| format!("ITaskDefinition::Triggers: {e}"))?;

        // Trigger 1: At logon of CURRENT user (required for non-admin Task Scheduler registration)
        let logon_base = triggers
            .Create(TASK_TRIGGER_LOGON)
            .map_err(|e| format!("Create(TASK_TRIGGER_LOGON): {e}"))?;
        let logon_trigger: ILogonTrigger = logon_base
            .cast()
            .map_err(|e| format!("cast ILogonTrigger: {e}"))?;
        let _ = logon_trigger.SetId(&BSTR::from("LogonTrigger"));
        let _ = logon_trigger.SetUserId(&user_bstr);
        let _ = logon_trigger.SetDelay(&BSTR::from("PT2S"));
        let _ = logon_trigger.SetEnabled(VARIANT_BOOL::from(true));

        // Trigger 2: On workstation unlock / wake from sleep for CURRENT user
        let unlock_base = triggers
            .Create(TASK_TRIGGER_SESSION_STATE_CHANGE)
            .map_err(|e| format!("Create(TASK_TRIGGER_SESSION_STATE_CHANGE): {e}"))?;
        let unlock_trigger: ISessionStateChangeTrigger = unlock_base
            .cast()
            .map_err(|e| format!("cast ISessionStateChangeTrigger: {e}"))?;
        let _ = unlock_trigger.SetId(&BSTR::from("UnlockTrigger"));
        let _ = unlock_trigger.SetUserId(&user_bstr);
        let _ = unlock_trigger.SetStateChange(TASK_SESSION_UNLOCK);
        let _ = unlock_trigger.SetDelay(&BSTR::from("PT1S"));
        let _ = unlock_trigger.SetEnabled(VARIANT_BOOL::from(true));

        let actions = task_def
            .Actions()
            .map_err(|e| format!("ITaskDefinition::Actions: {e}"))?;
        let exec_base = actions
            .Create(TASK_ACTION_EXEC)
            .map_err(|e| format!("Create(TASK_ACTION_EXEC): {e}"))?;
        let exec_action: IExecAction = exec_base
            .cast()
            .map_err(|e| format!("cast IExecAction: {e}"))?;

        let exe_str = exe_path.display().to_string();
        exec_action
            .SetPath(&BSTR::from(exe_str.as_str()))
            .map_err(|e| format!("IExecAction::SetPath: {e}"))?;
        exec_action
            .SetArguments(&BSTR::from("--apply-only"))
            .map_err(|e| format!("IExecAction::SetArguments: {e}"))?;
        if let Some(dir) = exe_path.parent() {
            let _ = exec_action.SetWorkingDirectory(&BSTR::from(dir.display().to_string().as_str()));
        }

        let user_var = VARIANT::from(user_bstr);
        root.RegisterTaskDefinition(
            &BSTR::from(STARTUP_TASK_NAME),
            &task_def,
            TASK_CREATE_OR_UPDATE.0,
            &user_var,
            &VARIANT::default(),
            TASK_LOGON_INTERACTIVE_TOKEN,
            &VARIANT::default(),
        )
        .map_err(|e| format!("RegisterTaskDefinition: {e}"))?;

        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_task_name_is_non_empty() {
        assert_eq!(STARTUP_TASK_NAME, "puwvga_ApplyColor");
        assert!(current_user_id().is_some());
    }

    #[test]
    #[ignore]
    fn live_verify_task_scheduler_non_admin_registration() {
        let exe = std::env::current_exe().unwrap();
        let res = set_apply_on_startup(true, &exe);
        println!("set_apply_on_startup(true) -> {:?}", res);
        assert!(res.is_ok(), "RegisterTaskDefinition failed: {:?}", res);
        assert!(is_apply_on_startup_enabled());
        let del = set_apply_on_startup(false, &exe);
        assert!(del.is_ok());
        assert!(!is_apply_on_startup_enabled());
    }
}

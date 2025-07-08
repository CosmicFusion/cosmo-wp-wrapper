use std::{cell::RefCell, fs::File, ops::DerefMut, os::unix::io::AsFd, rc::Rc};
use wayland_client::{
    delegate_dispatch, delegate_noop, event_created_child, protocol::{
        wl_buffer, wl_compositor, wl_keyboard, wl_registry, wl_seat, wl_shm, wl_shm_pool, wl_surface,
    }, Connection, Dispatch, QueueHandle,
};
use wayland_protocols::{ext::{data_control, workspace::v1::client::{
    ext_workspace_group_handle_v1::{self, ExtWorkspaceGroupHandleV1}, ext_workspace_handle_v1::{self, ExtWorkspaceHandleV1, State}, ext_workspace_manager_v1::{self, ExtWorkspaceManagerV1}
}}, xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base}};

 use wayland_protocols::ext::workspace::v1::client::ext_workspace_manager_v1::{
            EVT_WORKSPACE_GROUP_OPCODE, EVT_WORKSPACE_OPCODE,
        };

delegate_noop!(WorkspaceData: ignore ExtWorkspaceGroupHandleV1);

#[derive(Debug, Clone)]
struct CosmoWaylandWorkSpace {
    handle: ExtWorkspaceHandleV1,
    state: Option<wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::State>,
    name: String,
}

fn main() {
    let conn = Connection::connect_to_env().unwrap();
    let mut event_queue = conn.new_event_queue();
    let qhandle = event_queue.handle();

    let display = conn.display();
    display.get_registry(&qhandle, ());

    let mut workspace_data = WorkspaceData {
        workspace_fill_count: Rc::new(RefCell::new(0)),
        workspace_last_fill: Rc::new(RefCell::new(None)),
        workspace_manager: Rc::new(RefCell::new(None)),
        current_workspace: Rc::new(RefCell::new(None)),
        workspace_group: Rc::new(RefCell::new(None)),
        workspaces: Rc::new(RefCell::new(Vec::new())),
        workspace_handles: Rc::new(RefCell::new(Vec::new()))
    };

    // Continuously dispatch events until the "done" event is received
        event_queue.blocking_dispatch(&mut workspace_data).unwrap();
        event_queue.blocking_dispatch(&mut workspace_data).unwrap();

        let wk = &workspace_data.workspaces.borrow().clone()[3];
        let mut count = 0;
        while count < 1000 {
            wk.handle.activate();
            workspace_data.workspace_manager.borrow().clone().unwrap().commit();
            count += 1;
        }

    // When done is true, you can perform any final actions if necessary
    println!("Event loop finished.");
}

#[derive(Debug)]
struct WorkspaceData {
    workspace_fill_count: Rc<RefCell<usize>>,
    workspace_last_fill: Rc<RefCell<Option<CosmoWaylandWorkSpace>>>,
    workspace_manager: Rc<RefCell<Option<ExtWorkspaceManagerV1>>>,
    workspace_handles: Rc<RefCell<Vec<ExtWorkspaceHandleV1>>>,
    workspace_group: Rc<RefCell<Option<ExtWorkspaceGroupHandleV1>>>,
    workspaces: Rc<RefCell<Vec<CosmoWaylandWorkSpace>>>,
    current_workspace: Rc<RefCell<Option<CosmoWaylandWorkSpace>>>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for WorkspaceData {
    fn event(
        workspace_data: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global { name, interface, .. } = event {
            match &interface[..] {
                "ext_workspace_manager_v1" => {
                    let workspace_manager =
                        registry.bind::<ext_workspace_manager_v1::ExtWorkspaceManagerV1, _, _>(name, 1, qh, ());
                    *workspace_data.workspace_manager.borrow_mut() = Some(workspace_manager);
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<ext_workspace_manager_v1::ExtWorkspaceManagerV1, ()> for WorkspaceData {
    fn event(
        workspace_data: &mut Self,
        _manager: &ext_workspace_manager_v1::ExtWorkspaceManagerV1,
        event: ext_workspace_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        println!("Received event in workspace manager: {:?}", event);

        // Process workspace events and add them to workspace_data
        if let ext_workspace_manager_v1::Event::Workspace {  ref workspace, .. } = event {
            let mut workspace_handles = workspace_data.workspace_handles.borrow_mut();
            workspace_handles.push(workspace.clone());
        }

        if let ext_workspace_manager_v1::Event::WorkspaceGroup {  ref workspace_group, .. } = event {
            *workspace_data.workspace_group.borrow_mut() = Some(workspace_group.clone());
        }

        // Check for the "done" event and set done to true
        /*if let ext_workspace_manager_v1::Event::Done {} = event {
            println!("Done event received, setting done = true");
            workspace_data.done = true;  // Set done to true
        }*/
    }

    event_created_child!(WorkspaceData, ExtWorkspaceManagerV1, [
        EVT_WORKSPACE_OPCODE => (ExtWorkspaceHandleV1, ()),
        EVT_WORKSPACE_GROUP_OPCODE => (ExtWorkspaceGroupHandleV1, ()),
    ]);
}

// Handle events from workspace handles (if needed)
impl Dispatch<ExtWorkspaceHandleV1, ()> for WorkspaceData {
    fn event(
        workspace_data: &mut Self,
        _proxy: &ExtWorkspaceHandleV1,
        event: ext_workspace_handle_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let mut count = workspace_data.workspace_fill_count.borrow_mut();
        let workspace_handles = workspace_data.workspace_handles.borrow();
        match event {
            ext_workspace_handle_v1::Event::Name { name } => {
                let workspace_struct = CosmoWaylandWorkSpace {
                            handle: workspace_handles[count.clone()].clone(),
                            name: name,
                            state: None,
                };
                *workspace_data.workspace_last_fill.borrow_mut() = Some(workspace_struct);
            }
            ext_workspace_handle_v1::Event::State { state } => {
                let mut last_fill = workspace_data.workspace_last_fill.borrow_mut().clone();
                match last_fill {
                    Some(mut w) => {
                        w.state = match state.into_result() {
                            Ok(t) => Some(t),
                            Err(_) => None,
                        };
                        workspace_data.workspaces.borrow_mut().push(w.clone());
                        last_fill = None;
                        *count +=1 ;
                    }
                    None => {}
                }
            }
            _ => {}
        }
    }
}
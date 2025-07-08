use std::{cell::RefCell, rc::Rc, time::Duration};
use wayland_client::{
    delegate_noop, event_created_child,
    protocol::wl_registry,
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::ext::workspace::v1::client::{
            ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1,
            ext_workspace_handle_v1::{self, ExtWorkspaceHandleV1, State},
            ext_workspace_manager_v1::{self, ExtWorkspaceManagerV1},
        };

use wayland_protocols::ext::workspace::v1::client::ext_workspace_manager_v1::{
    EVT_WORKSPACE_GROUP_OPCODE, EVT_WORKSPACE_OPCODE,
};

delegate_noop!(WorkspaceData: ignore ExtWorkspaceGroupHandleV1);

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct CosmoWaylandWorkSpace {
    handle: ExtWorkspaceHandleV1,
    state: Option<wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::State>,
    name: String,
    id: u32,
}
#[warn(dead_code)]

fn main() {
    let args: Vec<String> = std::env::args().collect();

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
        workspace_handles: Rc::new(RefCell::new(Vec::new())),
        done: false,
    };

    // Continuously dispatch events until the "done" event is received
    while !workspace_data.done {
        event_queue.roundtrip(&mut workspace_data).unwrap();
    }

    assert!(workspace_data.done);

    let workspaces = workspace_data.workspaces.borrow().clone();
    
    if args.len() > 1 {
        match args[1].trim() {
            "get_active" => {
                println!("{}", workspace_data.current_workspace.borrow().clone().unwrap().id);
            }
            "switch" => {
                let target_workspace = if args.len() > 2 {
            let size = match args[2].parse::<i32>() {
                Ok(size) => {
                    size - 1
                }
                Err(_) => panic!("invalid argument: not int")
            };
            size
                } else {
                    panic!("args missing")
                };

                let workspace = if target_workspace > workspaces.len() as i32 {
                    panic!("invalid argument: int too big")  
                } else {
                    &workspaces[target_workspace as usize]
                };

                workspace.handle.activate();
                workspace_data.workspace_manager.borrow().clone().unwrap().commit();
                // Flush pending outgoing events to the server
                event_queue.flush().unwrap();
                std::thread::sleep(Duration::from_secs(1));
                event_queue.flush().unwrap();
            
            }
            _ => panic!("args missing")
        }
    } else {
        panic!("args missing")
    };
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
    done: bool,
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
                    let workspace_manager = registry
                        .bind::<ext_workspace_manager_v1::ExtWorkspaceManagerV1, _, _>(
                            name,
                            1,
                            qh,
                            (),
                        );
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

        // Process workspace events and add them to workspace_data
        if let ext_workspace_manager_v1::Event::Workspace { ref workspace, .. } = event {
            let mut workspace_handles = workspace_data.workspace_handles.borrow_mut();
            workspace_handles.push(workspace.clone());
        }

        if let ext_workspace_manager_v1::Event::WorkspaceGroup { ref workspace_group, .. } = event {
            *workspace_data.workspace_group.borrow_mut() = Some(workspace_group.clone());
        }

        // Check for the "done" event and set done to true
        if let ext_workspace_manager_v1::Event::Done {} = event {
            workspace_data.done = true; // Set done to true
        }
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
                    name,
                    id: count.clone() as u32 + 1,
                    state: None,
                };
                *workspace_data.workspace_last_fill.borrow_mut() = Some(workspace_struct);
            }
            ext_workspace_handle_v1::Event::State { state } => {
                let mut last_fill = workspace_data.workspace_last_fill.borrow_mut().clone();
                match last_fill {
                    Some(mut w) => {
                        w.state = match state.into_result() {
                            Ok(t) => {
                                if t == State::Active {
                                    *workspace_data.current_workspace.borrow_mut() = Some(w.clone());
                                }
                                Some(t)
                            },
                            Err(_) => None,
                        };
                        workspace_data.workspaces.borrow_mut().push(w.clone());
                        last_fill = None;
                        *count += 1;
                        assert!(last_fill.is_none())
                    }
                    None => {}
                }
            }
            _ => {}
        }
    }
}
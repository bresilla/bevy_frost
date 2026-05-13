use frost_core::{WorkspaceOwner, WorkspaceStack, WorkspaceStackError};

#[test]
fn workspace_stack_tracks_root_and_module_levels() {
    let mut stack = WorkspaceStack::new("root");

    assert_eq!(stack.depth(), 0);
    assert!(stack.is_root_active());
    assert!(matches!(stack.current().owner, WorkspaceOwner::Root));
    assert!(stack.current_policy().allow_app_window_controls);
    assert!(stack.current_policy().allow_root_ribbon);
    assert!(!stack.current_policy().allow_module_bars);
    assert!(stack.current_policy().allow_shelves);
    assert!(!stack.current_policy().inherit_root_shelves);

    let l1 = stack.push_module(egui::Id::new("graph"));
    assert_eq!(l1.depth, 1);
    assert_eq!(stack.depth(), 1);
    assert!(!stack.is_root_active());
    assert!(matches!(stack.current().owner, WorkspaceOwner::Module(_)));
    assert!(!stack.current_policy().allow_app_window_controls);
    assert!(!stack.current_policy().allow_root_ribbon);
    assert!(stack.current_policy().allow_module_bars);
    assert!(stack.current_policy().allow_shelves);
    assert!(stack.current_policy().inherit_root_shelves);
    assert!(stack.current_policy().restore_to_parent);

    let l2 = stack.push_module(egui::Id::new("image"));
    assert_eq!(l2.depth, 2);
    assert_eq!(stack.depth(), 2);

    let popped = stack.pop().expect("L2 can pop");
    assert_eq!(popped.depth, 2);
    assert_eq!(stack.depth(), 1);

    stack.pop().expect("L1 can pop");
    assert_eq!(stack.depth(), 0);
    assert_eq!(stack.pop(), Err(WorkspaceStackError::CannotPopRoot));
}

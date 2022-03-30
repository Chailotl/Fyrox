use crate::absm::{
    canvas::{AbsmCanvas, AbsmCanvasBuilder, AbsmCanvasMessage, Mode},
    command::{
        AbsmCommand, AbsmCommandStack, AbsmEditorContext, AddStateCommand, AddTransitionCommand,
        CommandGroup, MoveStateNodeCommand,
    },
    menu::Menu,
    message::AbsmMessage,
    node::{AbsmStateNode, AbsmStateNodeBuilder, AbsmStateNodeMessage},
    transition::{Transition, TransitionBuilder},
};
use fyrox::{
    animation::machine::{
        state::StateDefinition, transition::TransitionDefinition, MachineDefinition,
    },
    core::{algebra::Vector2, color::Color, pool::Handle},
    engine::Engine,
    gui::{
        border::BorderBuilder,
        grid::{Column, GridBuilder, Row},
        menu::{MenuItemBuilder, MenuItemContent, MenuItemMessage},
        message::{MessageDirection, UiMessage},
        popup::{Placement, PopupBuilder, PopupMessage},
        stack_panel::StackPanelBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
};
use std::{
    cmp::Ordering,
    sync::mpsc::{channel, Receiver, Sender},
};

mod canvas;
mod command;
mod menu;
mod message;
mod node;
mod transition;

const NORMAL_BACKGROUND: Color = Color::opaque(60, 60, 60);
const SELECTED_BACKGROUND: Color = Color::opaque(80, 80, 80);
const BORDER_COLOR: Color = Color::opaque(70, 70, 70);

pub struct AbsmEditor {
    #[allow(dead_code)] // TODO
    window: Handle<UiNode>,
    canvas_context_menu: CanvasContextMenu,
    node_context_menu: NodeContextMenu,
    command_stack: AbsmCommandStack,
    canvas: Handle<UiNode>,
    absm_definition: Option<MachineDefinition>,
    menu: Menu,
    message_sender: Sender<AbsmMessage>,
    message_receiver: Receiver<AbsmMessage>,
}

pub struct CanvasContextMenu {
    create_state: Handle<UiNode>,
    menu: Handle<UiNode>,
    pub canvas: Handle<UiNode>,
    pub node_context_menu: Handle<UiNode>,
}

impl CanvasContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_state;
        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_content(
                StackPanelBuilder::new(WidgetBuilder::new().with_child({
                    create_state = MenuItemBuilder::new(
                        WidgetBuilder::new().with_min_size(Vector2::new(120.0, 20.0)),
                    )
                    .with_content(MenuItemContent::text("Create State"))
                    .build(ctx);
                    create_state
                }))
                .build(ctx),
            )
            .build(ctx);

        Self {
            create_state,
            menu,
            canvas: Default::default(),
            node_context_menu: Default::default(),
        }
    }

    pub fn handle_ui_message(
        &mut self,
        sender: &Sender<AbsmMessage>,
        message: &UiMessage,
        ui: &mut UserInterface,
    ) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.create_state {
                let screen_position = ui.node(self.menu).screen_position();

                sender
                    .send(AbsmMessage::DoCommand(AbsmCommand::new(
                        AddStateCommand::new(StateDefinition {
                            position: ui.node(self.canvas).screen_to_local(screen_position),
                            name: "New State".to_string(),
                            root: Default::default(),
                        }),
                    )))
                    .unwrap();
            }
        }
    }
}

pub struct NodeContextMenu {
    create_transition: Handle<UiNode>,
    menu: Handle<UiNode>,
    canvas: Handle<UiNode>,
    placement_target: Handle<UiNode>,
}

impl NodeContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_transition;
        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_content(
                StackPanelBuilder::new(WidgetBuilder::new().with_child({
                    create_transition = MenuItemBuilder::new(
                        WidgetBuilder::new().with_min_size(Vector2::new(120.0, 20.0)),
                    )
                    .with_content(MenuItemContent::text("Create Transition"))
                    .build(ctx);
                    create_transition
                }))
                .build(ctx),
            )
            .build(ctx);

        Self {
            create_transition,
            menu,
            canvas: Default::default(),
            placement_target: Default::default(),
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &mut UserInterface) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.create_transition {
                assert!(ui
                    .node(self.placement_target)
                    .query_component::<AbsmStateNode>()
                    .is_some());

                ui.send_message(AbsmCanvasMessage::switch_mode(
                    self.canvas,
                    MessageDirection::ToWidget,
                    Mode::CreateTransition {
                        source: self.placement_target,
                        source_pos: ui.node(self.placement_target).center(),
                        dest_pos: ui.node(self.canvas).screen_to_local(ui.cursor_position()),
                    },
                ))
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu {
                self.placement_target = *target;
            }
        }
    }
}

fn fetch_state_node_model_handle(
    handle: Handle<UiNode>,
    ui: &UserInterface,
) -> Handle<StateDefinition> {
    ui.node(handle)
        .query_component::<AbsmStateNode>()
        .unwrap()
        .model_handle
}

impl AbsmEditor {
    pub fn new(ui: &mut UserInterface) -> Self {
        let (tx, rx) = channel();

        let ctx = &mut ui.build_ctx();
        let mut node_context_menu = NodeContextMenu::new(ctx);
        let mut canvas_context_menu = CanvasContextMenu::new(ctx);
        let menu = Menu::new(ctx);

        let canvas;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(700.0).with_height(400.0))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new().with_child(menu.menu).with_child(
                        BorderBuilder::new(WidgetBuilder::new().on_row(1).with_child({
                            canvas = AbsmCanvasBuilder::new(
                                WidgetBuilder::new().with_context_menu(canvas_context_menu.menu),
                            )
                            .build(ctx);
                            canvas
                        }))
                        .build(ctx),
                    ),
                )
                .add_row(Row::strict(24.0))
                .add_row(Row::stretch())
                .add_column(Column::stretch())
                .build(ctx),
            )
            .with_title(WindowTitle::text("ABSM Editor"))
            .build(ctx);

        canvas_context_menu.canvas = canvas;
        canvas_context_menu.node_context_menu = node_context_menu.menu;
        node_context_menu.canvas = canvas;

        let mut absm_definition = MachineDefinition::default();

        let state1 = absm_definition.states.spawn(StateDefinition {
            position: Default::default(),
            name: "State".to_string(),
            root: Default::default(),
        });
        let state2 = absm_definition.states.spawn(StateDefinition {
            position: Vector2::new(300.0, 200.0),
            name: "Other State".to_string(),
            root: Default::default(),
        });
        let _ = absm_definition.transitions.spawn(TransitionDefinition {
            name: "Transition".to_string(),
            transition_time: 0.2,
            source: state1,
            dest: state2,
            rule: "Rule1".to_string(),
        });

        let mut editor = Self {
            window,
            canvas_context_menu,
            node_context_menu,
            message_sender: tx,
            message_receiver: rx,
            command_stack: AbsmCommandStack::new(false),
            canvas,
            absm_definition: Some(absm_definition),
            menu,
        };

        editor.sync_to_model(ui);

        editor
    }

    fn sync_to_model(&mut self, ui: &mut UserInterface) {
        if let Some(definition) = self.absm_definition.as_ref() {
            let canvas = ui
                .node(self.canvas)
                .cast::<AbsmCanvas>()
                .expect("Must be AbsmCanvas!");

            let mut states = canvas
                .children()
                .iter()
                .cloned()
                .filter(|c| ui.node(*c).has_component::<AbsmStateNode>())
                .collect::<Vec<_>>();

            let mut transitions = canvas
                .children()
                .iter()
                .cloned()
                .filter(|c| ui.node(*c).has_component::<Transition>())
                .collect::<Vec<_>>();

            match states
                .len()
                .cmp(&(definition.states.alive_count() as usize))
            {
                Ordering::Less => {
                    // A state was added.
                    for (state_handle, state) in definition.states.pair_iter() {
                        if states.iter().all(|state_view| {
                            ui.node(*state_view)
                                .query_component::<AbsmStateNode>()
                                .unwrap()
                                .model_handle
                                != state_handle
                        }) {
                            let state_view_handle = AbsmStateNodeBuilder::new(
                                WidgetBuilder::new()
                                    .with_context_menu(self.node_context_menu.menu)
                                    .with_desired_position(state.position),
                            )
                            .with_name(state.name.clone())
                            .build(state_handle, &mut ui.build_ctx());

                            states.push(state_view_handle);

                            ui.send_message(WidgetMessage::link(
                                state_view_handle,
                                MessageDirection::ToWidget,
                                self.canvas,
                            ));
                        }
                    }
                }
                Ordering::Greater => {
                    // A state was removed.
                    for (state_view_handle, state_model_handle) in
                        states.clone().iter().cloned().map(|state_view| {
                            (
                                state_view,
                                ui.node(state_view)
                                    .query_component::<AbsmStateNode>()
                                    .unwrap()
                                    .model_handle,
                            )
                        })
                    {
                        if definition
                            .states
                            .pair_iter()
                            .all(|(h, _)| h != state_model_handle)
                        {
                            ui.send_message(WidgetMessage::remove(
                                state_view_handle,
                                MessageDirection::ToWidget,
                            ));

                            if let Some(position) =
                                states.iter().position(|s| *s == state_view_handle)
                            {
                                states.remove(position);
                            }
                        }
                    }
                }
                _ => (),
            }

            // Sync state nodes.
            for state in states.iter() {
                let state_node = ui.node(*state).query_component::<AbsmStateNode>().unwrap();
                let state_model_ref = &definition.states[state_node.model_handle];

                if state_model_ref.name != state_node.name {
                    ui.send_message(AbsmStateNodeMessage::name(
                        *state,
                        MessageDirection::ToWidget,
                        state_node.name.clone(),
                    ));
                }

                ui.send_message(WidgetMessage::desired_position(
                    *state,
                    MessageDirection::ToWidget,
                    state_model_ref.position,
                ));
            }

            // Force update layout to be able to fetch positions of nodes for transitions.
            ui.update(ui.screen_size(), 0.0);

            // Sync transitions.
            match transitions
                .len()
                .cmp(&(definition.transitions.alive_count() as usize))
            {
                Ordering::Less => {
                    // A transition was added.
                    for (transition_handle, transition) in definition.transitions.pair_iter() {
                        if transitions.iter().all(|transition_view| {
                            ui.node(*transition_view)
                                .query_component::<Transition>()
                                .unwrap()
                                .model_handle
                                != transition_handle
                        }) {
                            fn find_state_view(
                                state_handle: Handle<StateDefinition>,
                                states: &[Handle<UiNode>],
                                ui: &UserInterface,
                            ) -> Handle<UiNode> {
                                states
                                    .iter()
                                    .find(|s| {
                                        ui.node(**s)
                                            .query_component::<AbsmStateNode>()
                                            .unwrap()
                                            .model_handle
                                            == state_handle
                                    })
                                    .cloned()
                                    .unwrap_or_default()
                            }

                            let transition_view = TransitionBuilder::new(WidgetBuilder::new())
                                .with_source(find_state_view(transition.source, &states, ui))
                                .with_dest(find_state_view(transition.dest, &states, ui))
                                .build(transition_handle, &mut ui.build_ctx());

                            ui.send_message(WidgetMessage::link(
                                transition_view,
                                MessageDirection::ToWidget,
                                self.canvas,
                            ));

                            ui.send_message(WidgetMessage::lowermost(
                                transition_view,
                                MessageDirection::ToWidget,
                            ));

                            transitions.push(transition_view);
                        }
                    }
                }

                Ordering::Greater => {
                    // A transition was removed.
                    for (transition_view_handle, transition_model_handle) in
                        transitions.clone().iter().cloned().map(|transition_view| {
                            (
                                transition_view,
                                ui.node(transition_view)
                                    .query_component::<Transition>()
                                    .unwrap()
                                    .model_handle,
                            )
                        })
                    {
                        if definition
                            .transitions
                            .pair_iter()
                            .all(|(h, _)| h != transition_model_handle)
                        {
                            ui.send_message(WidgetMessage::remove(
                                transition_view_handle,
                                MessageDirection::ToWidget,
                            ));

                            if let Some(position) = transitions
                                .iter()
                                .position(|s| *s == transition_view_handle)
                            {
                                transitions.remove(position);
                            }
                        }
                    }
                }
                Ordering::Equal => {}
            }
        }
    }

    fn do_command(&mut self, command: AbsmCommand) -> bool {
        if let Some(definition) = self.absm_definition.as_mut() {
            self.command_stack
                .do_command(command.into_inner(), AbsmEditorContext { definition });
            true
        } else {
            false
        }
    }

    fn undo_command(&mut self) -> bool {
        if let Some(definition) = self.absm_definition.as_mut() {
            self.command_stack.undo(AbsmEditorContext { definition });
            true
        } else {
            false
        }
    }

    fn redo_command(&mut self) -> bool {
        if let Some(definition) = self.absm_definition.as_mut() {
            self.command_stack.redo(AbsmEditorContext { definition });
            true
        } else {
            false
        }
    }

    fn clear_command_stack(&mut self) -> bool {
        if let Some(definition) = self.absm_definition.as_mut() {
            self.command_stack.clear(AbsmEditorContext { definition });
            true
        } else {
            false
        }
    }

    pub fn update(&mut self, engine: &mut Engine) {
        let mut need_sync = false;

        while let Ok(message) = self.message_receiver.try_recv() {
            match message {
                AbsmMessage::DoCommand(command) => {
                    need_sync |= self.do_command(command);
                }
                AbsmMessage::Undo => {
                    need_sync |= self.undo_command();
                }
                AbsmMessage::Redo => {
                    need_sync |= self.redo_command();
                }
                AbsmMessage::ClearCommandStack => {
                    need_sync |= self.clear_command_stack();
                }
            }
        }

        if need_sync {
            self.sync_to_model(&mut engine.user_interface);
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &mut UserInterface) {
        self.menu.handle_ui_message(&self.message_sender, message);
        self.node_context_menu.handle_ui_message(message, ui);
        self.canvas_context_menu
            .handle_ui_message(&self.message_sender, message, ui);

        if let Some(msg) = message.data::<AbsmCanvasMessage>() {
            match msg {
                AbsmCanvasMessage::CommitTransition { source, dest } => {
                    if message.destination() == self.canvas
                        && message.direction() == MessageDirection::FromWidget
                    {
                        let source = fetch_state_node_model_handle(*source, ui);
                        let dest = fetch_state_node_model_handle(*dest, ui);

                        self.message_sender
                            .send(AbsmMessage::DoCommand(AbsmCommand::new(
                                AddTransitionCommand::new(TransitionDefinition {
                                    name: "Transition".to_string(),
                                    transition_time: 1.0,
                                    source,
                                    dest,
                                    rule: "".to_string(),
                                }),
                            )))
                            .unwrap();
                    }
                }
                AbsmCanvasMessage::CommitDrag { entries } => {
                    let commands = entries
                        .iter()
                        .map(|e| {
                            let state_handle = fetch_state_node_model_handle(e.node, ui);
                            let new_position = ui.node(e.node).actual_local_position();

                            AbsmCommand::new(MoveStateNodeCommand::new(
                                state_handle,
                                e.initial_position,
                                new_position,
                            ))
                        })
                        .collect::<Vec<_>>();

                    self.message_sender
                        .send(AbsmMessage::DoCommand(AbsmCommand::new(
                            CommandGroup::from(commands),
                        )))
                        .unwrap();
                }
                _ => (),
            }
        }
    }
}
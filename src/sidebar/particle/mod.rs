use crate::scene::commands::particle_system::{
    AddParticleSystemEmitterCommand, DeleteEmitterCommand, SetParticleSystemAccelerationCommand,
};
use crate::scene::commands::SceneCommand;
use crate::{
    gui::{
        BuildContext, DeletableItemBuilder, DeletableItemMessage, EditorUiMessage, EditorUiNode,
        Ui, UiMessage, UiNode,
    },
    load_image, send_sync_message,
    sidebar::{
        make_text_mark, make_vec3_input_field, particle::emitter::EmitterSection, COLUMN_WIDTH,
        ROW_HEIGHT,
    },
    Message,
};
use rg3d::{
    core::{pool::Handle, scope_profile},
    gui::{
        button::ButtonBuilder,
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{
            ButtonMessage, DropdownListMessage, MessageDirection, UiMessageData, Vec3EditorMessage,
            WidgetMessage,
        },
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        HorizontalAlignment, Orientation, Thickness, VerticalAlignment,
    },
    scene::{
        node::Node,
        particle_system::{
            BaseEmitterBuilder, BoxEmitterBuilder, CylinderEmitterBuilder, Emitter,
            SphereEmitterBuilder,
        },
    },
};
use std::sync::mpsc::Sender;

mod cuboid;
mod cylinder;
mod emitter;
mod sphere;

pub struct ParticleSystemSection {
    pub section: Handle<UiNode>,
    acceleration: Handle<UiNode>,
    add_box_emitter: Handle<UiNode>,
    add_sphere_emitter: Handle<UiNode>,
    add_cylinder_emitter: Handle<UiNode>,
    emitters: Handle<UiNode>,
    sender: Sender<Message>,
    emitter_index: Option<usize>,
    emitter_section: EmitterSection,
}

fn make_button_image(ctx: &mut BuildContext, image_data: &[u8]) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_width(ROW_HEIGHT - 11.0)
            .with_height(ROW_HEIGHT - 11.0)
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_content(
        ImageBuilder::new(WidgetBuilder::new())
            .with_opt_texture(load_image(image_data))
            .build(ctx),
    )
    .build(ctx)
}

impl ParticleSystemSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let emitter_section = EmitterSection::new(ctx, sender.clone());

        let acceleration;
        let emitters;
        let add_box_emitter;
        let add_sphere_emitter;
        let add_cylinder_emitter;
        let section = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child(make_text_mark(ctx, "Acceleration", 0))
                            .with_child({
                                acceleration = make_vec3_input_field(ctx, 0);
                                acceleration
                            })
                            .with_child(make_text_mark(ctx, "Emitters", 1))
                            .with_child(
                                GridBuilder::new(
                                    WidgetBuilder::new()
                                        .on_row(1)
                                        .on_column(1)
                                        .with_child(
                                            StackPanelBuilder::new(
                                                WidgetBuilder::new()
                                                    .on_row(0)
                                                    .with_child(
                                                        TextBuilder::new(WidgetBuilder::new())
                                                            .with_text("Add Emitter: ")
                                                            .with_vertical_text_alignment(
                                                                VerticalAlignment::Center,
                                                            )
                                                            .build(ctx),
                                                    )
                                                    .with_child({
                                                        add_box_emitter = make_button_image(
                                                            ctx,
                                                            include_bytes!("../../../resources/embed/add_box_emitter.png"),
                                                        );
                                                        add_box_emitter
                                                    })
                                                    .with_child({
                                                        add_sphere_emitter = make_button_image(
                                                            ctx,
                                                            include_bytes!("../../../resources/embed/add_sphere_emitter.png"),
                                                        );
                                                        add_sphere_emitter
                                                    })
                                                    .with_child({
                                                        add_cylinder_emitter = make_button_image(
                                                            ctx,
                                                         include_bytes!("../../../resources/embed/add_cylinder_emitter.png"),
                                                        );
                                                        add_cylinder_emitter
                                                    }),
                                            )
                                            .with_orientation(Orientation::Horizontal)
                                            .build(ctx),
                                        )
                                        .with_child({
                                            emitters = DropdownListBuilder::new(
                                                WidgetBuilder::new().on_row(1),
                                            )
                                            .with_close_on_selection(true)
                                            .build(ctx);
                                            emitters
                                        }),
                                )
                                .add_row(Row::strict(ROW_HEIGHT))
                                .add_row(Row::strict(ROW_HEIGHT))
                                .add_column(Column::stretch())
                                .build(ctx),
                            ),
                    )
                    .add_column(Column::strict(COLUMN_WIDTH))
                    .add_column(Column::stretch())
                    .add_row(Row::strict(ROW_HEIGHT))
                    .add_row(Row::strict(ROW_HEIGHT * 2.0))
                    .build(ctx),
                )
                .with_child(emitter_section.section),
        )
        .build(ctx);

        Self {
            section,
            acceleration,
            add_box_emitter,
            add_sphere_emitter,
            add_cylinder_emitter,
            emitters,
            sender,
            emitter_index: None,
            emitter_section,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut Ui) {
        send_sync_message(
            ui,
            WidgetMessage::visibility(
                self.section,
                MessageDirection::ToWidget,
                node.is_particle_system(),
            ),
        );

        if let Node::ParticleSystem(particle_system) = node {
            send_sync_message(
                ui,
                Vec3EditorMessage::value(
                    self.acceleration,
                    MessageDirection::ToWidget,
                    particle_system.acceleration(),
                ),
            );

            let ctx = &mut ui.build_ctx();
            let emitters = particle_system
                .emitters
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let item = DeletableItemBuilder::new(WidgetBuilder::new())
                        .with_content(
                            TextBuilder::new(WidgetBuilder::new())
                                .with_text(match e {
                                    Emitter::Unknown => unreachable!(),
                                    Emitter::Box(_) => "Box",
                                    Emitter::Sphere(_) => "Sphere",
                                    Emitter::Cylinder(_) => "Cylinder",
                                })
                                .with_horizontal_text_alignment(HorizontalAlignment::Center)
                                .with_vertical_text_alignment(VerticalAlignment::Center)
                                .build(ctx),
                        )
                        .with_data(i)
                        .build(ctx);
                    ctx.add_node(UiNode::User(EditorUiNode::EmitterItem(item)))
                })
                .collect::<Vec<_>>();

            // Try to keep selection.
            if let Some(emitter_index) = self.emitter_index {
                if emitter_index >= particle_system.emitters.len() {
                    self.emitter_index = None;
                }
            } else if !particle_system.emitters.is_empty() {
                self.emitter_index = Some(0);
            }

            send_sync_message(
                ui,
                DropdownListMessage::items(self.emitters, MessageDirection::ToWidget, emitters),
            );

            send_sync_message(
                ui,
                DropdownListMessage::selection(
                    self.emitters,
                    MessageDirection::ToWidget,
                    self.emitter_index,
                ),
            );

            if let Some(emitter_index) = self.emitter_index {
                self.emitter_section
                    .sync_to_model(&particle_system.emitters[emitter_index], ui);
            }
        }
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        node: &Node,
        handle: Handle<Node>,
        ui: &Ui,
    ) {
        scope_profile!();

        if let Node::ParticleSystem(particle_system) = node {
            if let Some(emitter_index) = self.emitter_index {
                self.emitter_section.handle_message(
                    message,
                    &particle_system.emitters[emitter_index],
                    emitter_index,
                    handle,
                );
            }

            match *message.data() {
                UiMessageData::Vec3Editor(Vec3EditorMessage::Value(value)) => {
                    if particle_system.acceleration() != value
                        && message.destination() == self.acceleration
                    {
                        self.sender
                            .send(Message::DoSceneCommand(
                                SceneCommand::SetParticleSystemAcceleration(
                                    SetParticleSystemAccelerationCommand::new(handle, value),
                                ),
                            ))
                            .unwrap();
                    }
                }
                UiMessageData::Button(ButtonMessage::Click) => {
                    if message.destination() == self.add_box_emitter {
                        self.sender
                            .send(Message::DoSceneCommand(
                                SceneCommand::AddParticleSystemEmitter(
                                    AddParticleSystemEmitterCommand::new(
                                        handle,
                                        BoxEmitterBuilder::new(BaseEmitterBuilder::new()).build(),
                                    ),
                                ),
                            ))
                            .unwrap();
                    } else if message.destination() == self.add_sphere_emitter {
                        self.sender
                            .send(Message::DoSceneCommand(
                                SceneCommand::AddParticleSystemEmitter(
                                    AddParticleSystemEmitterCommand::new(
                                        handle,
                                        SphereEmitterBuilder::new(BaseEmitterBuilder::new())
                                            .build(),
                                    ),
                                ),
                            ))
                            .unwrap();
                    } else if message.destination() == self.add_cylinder_emitter {
                        self.sender
                            .send(Message::DoSceneCommand(
                                SceneCommand::AddParticleSystemEmitter(
                                    AddParticleSystemEmitterCommand::new(
                                        handle,
                                        CylinderEmitterBuilder::new(BaseEmitterBuilder::new())
                                            .build(),
                                    ),
                                ),
                            ))
                            .unwrap();
                    }
                }
                UiMessageData::User(EditorUiMessage::DeletableItem(
                    DeletableItemMessage::Delete,
                )) => {
                    if ui
                        .node(self.emitters)
                        .as_dropdown_list()
                        .items()
                        .contains(&message.destination())
                    {
                        if let UiNode::User(EditorUiNode::EmitterItem(ei)) =
                            ui.node(message.destination())
                        {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::DeleteEmitter(
                                    DeleteEmitterCommand::new(handle, ei.data.unwrap()),
                                )))
                                .unwrap();
                        } else {
                            unreachable!()
                        }
                    }
                }
                UiMessageData::DropdownList(DropdownListMessage::SelectionChanged(selection)) => {
                    if message.destination() == self.emitters {
                        self.emitter_index = selection;
                        self.sender.send(Message::SyncToModel).unwrap();
                    }
                }
                _ => {}
            }
        }
    }
}

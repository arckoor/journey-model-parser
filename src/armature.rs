use serde::Serialize;

use crate::{
    error::ParseError,
    structure::{Node, NodeKind, Transform},
    Options,
};

#[derive(Serialize, Debug)]
pub struct Armature {
    pub root: Joint,
}

#[derive(Serialize, Debug)]
pub struct Joint {
    pub name: String,
    pub children: Vec<Joint>,
    pub transform: Transform,
    pub aliases: Vec<String>,
}

impl Armature {
    pub fn new(options: &Options, root_node: &NodeKind) -> Result<Option<Self>, ParseError> {
        let transform = if let NodeKind::Root(root_node) = root_node {
            root_node.transform.clone()
        } else {
            return Err(ParseError::new("Expected root node"));
        };

        let mut children = Self::flatten(root_node, Vec::new(), options.armature_full);
        if children.is_empty() {
            return Ok(None);
        }

        if children.len() == 1 {
            return Ok(Some(Self {
                root: children.remove(0),
            }));
        }

        let root = Joint {
            name: "root".to_string(),
            children,
            transform,
            aliases: Vec::new(),
        };

        Ok(Some(Self { root }))
    }

    fn flatten(node: &NodeKind, mut aliases: Vec<String>, full: bool) -> Vec<Joint> {
        let mut joint_nodes = Vec::new();

        let mut process_joint = |node: &Node| {
            let children = node
                .nodes
                .iter()
                .flat_map(|child| Self::flatten(child, Vec::new(), full))
                .collect();
            joint_nodes.push(Joint {
                name: node.id.clone(),
                children,
                transform: node.transform.clone(),
                aliases: aliases.clone(),
            });
        };

        match node {
            NodeKind::Joint(joint) => {
                process_joint(joint);
            }
            NodeKind::Root(node) | NodeKind::Render(node) | NodeKind::Node(node) => {
                if full {
                    process_joint(node);
                } else {
                    aliases.push(node.id.clone());
                    joint_nodes.extend(
                        node.nodes
                            .iter()
                            .flat_map(|child| Self::flatten(child, aliases.clone(), full)),
                    );
                }
            }
        }

        joint_nodes
    }
}

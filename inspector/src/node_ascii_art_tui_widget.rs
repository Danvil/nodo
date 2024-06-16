// Copyright 2022 by David Weikersdorfer
use crate::nodo::inspector as nodi;
use crate::tui_style::TuiStyle;
use inspector_proto::uri_path_last;
use tui::{buffer::Buffer, layout::Rect, widgets::Widget};

#[derive(Debug, Clone)]
struct ChannelInfo {
    name: String,
    is_alias: bool,
    num_available: i64,
}

#[derive(Debug, Clone)]
pub struct NodeAsciiArtTuiWidget<'a> {
    name: String,
    is_executable: bool,
    rx: Vec<ChannelInfo>,
    tx: Vec<ChannelInfo>,
    style: &'a TuiStyle,
}

impl<'a> NodeAsciiArtTuiWidget<'a> {
    pub fn new(
        world: &nodi::Worldstate,
        vertex: &nodi::Vertex,
        style: &'a TuiStyle,
    ) -> NodeAsciiArtTuiWidget<'a> {
        NodeAsciiArtTuiWidget {
            name: uri_path_last(&vertex.name).unwrap_or(String::from("")),
            is_executable: vertex.is_executable,
            rx: world
                .vertex_rx_channels(vertex)
                .iter()
                .map(|(_, s, c)| ChannelInfo {
                    name: s.clone(),
                    is_alias: c.is_alias,
                    num_available: c.num_available,
                })
                .collect(),
            tx: world
                .vertex_tx_channels(vertex)
                .iter()
                .map(|(_, s, c)| ChannelInfo {
                    name: s.clone(),
                    is_alias: c.is_alias,
                    num_available: c.num_available,
                })
                .collect(),
            style: style,
        }
    }
}

impl<'a> Widget for NodeAsciiArtTuiWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let num_rx = self.rx.len();
        let num_tx = self.tx.len();
        let num_max = std::cmp::max(num_rx, num_tx);

        let node_width = 20;
        let node_height = 2 + 2 * num_max as u16;
        let node_pad_left = 1;
        let node_pad_right = 1;

        let pad_left = area.width.saturating_sub(node_width) / 2;
        let pad_top = area.height.saturating_sub(node_height) / 2;

        for i in 0..node_height {
            buf.get_mut(
                area.left() + pad_left + node_pad_left,
                area.top() + pad_top + i,
            )
            .set_symbol("│");
            buf.get_mut(
                area.left() + pad_left + node_pad_left + node_width - 1,
                area.top() + pad_top + i,
            )
            .set_symbol("│");
        }
        for i in 0..node_width {
            buf.get_mut(
                area.left() + pad_left + node_pad_left + i,
                area.top() + pad_top,
            )
            .set_symbol("─");
            buf.get_mut(
                area.left() + pad_left + node_pad_left + i,
                area.top() + pad_top + node_height,
            )
            .set_symbol("─");
        }
        {
            buf.get_mut(area.left() + pad_left + node_pad_left, area.top() + pad_top)
                .set_symbol("┌");
            buf.get_mut(
                area.left() + pad_left + node_pad_left + node_width - 1,
                area.top() + pad_top,
            )
            .set_symbol("┐");
            buf.get_mut(
                area.left() + pad_left + node_pad_left + node_width - 1,
                area.top() + pad_top + node_height,
            )
            .set_symbol("┘");
            buf.get_mut(
                area.left() + pad_left + node_pad_left,
                area.top() + pad_top + node_height,
            )
            .set_symbol("└");
        }

        let rx_top_offset = num_max.saturating_sub(num_rx) as u16;
        let tx_top_offset = num_max.saturating_sub(num_tx) as u16;

        for i in 0..self.rx.len() {
            let rx = &self.rx[i];

            let channel_style = if rx.is_alias {
                self.style.channel_alias
            } else if rx.num_available > 0 {
                self.style.channel_available
            } else {
                self.style.channel_empty
            };

            buf.set_string(
                area.left() + pad_left - rx.name.len() as u16,
                area.top() + pad_top + 2 + 2 * (i as u16) + rx_top_offset,
                &self.rx[i].name,
                channel_style,
            );
            buf.set_string(
                area.left() + pad_left + node_pad_left,
                area.top() + pad_top + 2 + 2 * (i as u16) + rx_top_offset,
                ">RX>",
                if rx.is_alias {
                    self.style.ascii_art_node_alias_channel_tag
                } else {
                    self.style.ascii_art_node_channel_tag
                },
            );
        }

        for i in 0..self.tx.len() {
            let tx = &self.tx[i];

            let channel_style = if tx.is_alias {
                self.style.channel_alias
            } else if tx.num_available > 0 {
                self.style.channel_available
            } else {
                self.style.channel_empty
            };

            buf.set_string(
                area.left() + pad_left + node_pad_left + node_width + node_pad_right,
                area.top() + pad_top + 2 + 2 * (i as u16) + tx_top_offset,
                &tx.name,
                channel_style,
            );
            buf.set_string(
                area.left() + pad_left + node_pad_left + node_width - 4,
                area.top() + pad_top + 2 + 2 * (i as u16) + tx_top_offset,
                ">TX>",
                if tx.is_alias {
                    self.style.ascii_art_node_alias_channel_tag
                } else {
                    self.style.ascii_art_node_channel_tag
                },
            );
        }

        buf.set_string(
            area.left() + pad_left + node_pad_left + 1,
            area.top() + pad_top,
            self.name,
            if self.is_executable {
                self.style.default_text
            } else {
                self.style.default_text_low
            },
        );
    }
}

use ggez::{Context, GameError};
use ggez::graphics::{Canvas, Color, DrawMode, Mesh, Rect};
use crate::chunk_viewer::viewport::Viewport;

pub struct Sidebar {
    visible: bool,
    mask: i32,
    natural_hash: i32,
    clustered_hash: i32
}
impl Sidebar {
    pub fn new(visible: bool) -> Sidebar {
        Sidebar { visible, mask: 0, natural_hash: 0, clustered_hash: 0 }
    }

    pub fn render(&mut self, viewport: &Viewport, ctx: &Context, canvas: &mut Canvas) -> Result<(), GameError> {
        if !self.visible {
            return Ok(());
        }

        let sidebar_bg = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(viewport.screen_width - 24.0, 0.0, 24.0, viewport.screen_height),
            Color::BLACK
        )?;
        canvas.draw(&sidebar_bg, [0.0, 0.0]);

        let chunk_highlight_height = 6.0;

        let natural_y = (self.natural_hash as f32 / (self.mask as f32 + 1.0)) * viewport.screen_height - (chunk_highlight_height / 2.0);
        let natural_hash_mesh = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(viewport.screen_width - 20.0, natural_y, 20.0, chunk_highlight_height),
            Color::from_rgba(0, 255, 255, 90)
        )?;

        let clustered_y = (self.clustered_hash as f32 / (self.mask as f32 + 1.0)) * viewport.screen_height - (chunk_highlight_height / 2.0);
        let clustered_hash_mesh = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(viewport.screen_width - 20.0, clustered_y, 20.0, chunk_highlight_height),
            Color::CYAN
        )?;

        // Wrap around
        if self.clustered_hash < self.natural_hash {
            let cluster_mesh_1 = Mesh::new_rectangle(
                ctx,
                DrawMode::fill(),
                Rect::new(viewport.screen_width - 20.0, natural_y, 20.0, viewport.screen_height - natural_y),
                Color::BLUE
            )?;
            canvas.draw(&cluster_mesh_1, [0.0, 0.0]);

            let cluster_mesh_2 = Mesh::new_rectangle(
                ctx,
                DrawMode::fill(),
                Rect::new(viewport.screen_width - 20.0, 0.0, 20.0, clustered_y),
                Color::BLUE
            )?;
            canvas.draw(&cluster_mesh_2, [0.0, 0.0]);
        } else {
            let cluster_mesh_1 = Mesh::new_rectangle(
                ctx,
                DrawMode::fill(),
                Rect::new(viewport.screen_width - 20.0, natural_y, 20.0, clustered_y - natural_y),
                Color::BLUE
            )?;
            canvas.draw(&cluster_mesh_1, [0.0, 0.0]);
        }

        canvas.draw(&natural_hash_mesh, [0.0, 0.0]);
        canvas.draw(&clustered_hash_mesh, [0.0, 0.0]);

        Ok(())
    }

    pub fn set_mask(&mut self, mask: i32) {
        self.mask = mask;
    }

    pub fn set_hashes(&mut self, natural_hash: i32, clustered_hash: i32) {
        self.natural_hash = natural_hash;
        self.clustered_hash = clustered_hash;
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }
}
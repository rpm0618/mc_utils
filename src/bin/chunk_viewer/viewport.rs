use std::ops::{Add, Div, Mul, Sub, SubAssign};
use ggez::glam::{Vec2, vec2};
use ggez::graphics::Rect;
use ggez::mint::Point2;
use mc_utils::positions::ChunkPos;

#[derive(PartialEq, Clone)]
pub struct Viewport {
    pub screen_width: f32,
    pub screen_height: f32,
    pub scale_factor: f32, // DPI Scale

    pub view_origin: Vec2,
    pub zoom: f32, // User Zoom

    pub chunk_size: f32
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            screen_width: 800.0,
            screen_height: 600.0,
            scale_factor: 1.0,

            view_origin: vec2(-400.0, -300.0),
            zoom: 1.0,

            chunk_size: 5.0
        }
    }

    pub fn zoom_into<P>(&mut self, dz: f32, zoom_origin: P)
        where P: Into<Point2<f32>>
    {
        let zoom_origin = self.scale_point(zoom_origin);
        let zoom_ratio = (self.scale_factor * (self.zoom + dz)) / (self.scale_factor * self.zoom);

        self.zoom += dz;
        if self.zoom < 0.1 {
            self.zoom = 0.1;
            return;
        }

        let new_zoom_origin = zoom_origin.mul(zoom_ratio);
        let offset = zoom_origin.sub(new_zoom_origin).div(zoom_ratio);
        self.view_origin.sub_assign(offset);
    }

    pub fn translate<P>(&mut self, delta: P)
        where P: Into<Point2<f32>>
    {
        let delta = self.scale_point(delta);
        self.view_origin.sub_assign(delta);
    }

    pub fn on_resize(&mut self, width: f32, height: f32, scale_factor: f32) {
        self.screen_width = width;
        self.screen_height = height;
        self.scale_factor = scale_factor;
    }

    pub fn get_screen_coordinates(&self) -> Rect {
        let scale = self.scale_factor * self.zoom;
        Rect {
            x: self.view_origin.x, y: self.view_origin.y,
            w: self.screen_width / scale, h: self.screen_height / scale
        }
    }

    pub fn scale_point<P>(&self, point: P) -> Vec2
        where P: Into<Point2<f32>>
    {
        let scale = self.scale_factor * self.zoom;
        let p = point.into();
        vec2(p.x / scale, p.y / scale)
    }

    pub fn transform_point<P>(&self, point: P) -> Vec2
        where P: Into<Point2<f32>> {
        self.scale_point(point).add(self.view_origin)
    }

    pub fn chunk_at<P>(&self, point: P) -> ChunkPos
        where P: Into<Point2<f32>>
    {
        let pos = self.transform_point(point).div(self.chunk_size).floor();
        ChunkPos {
            x: pos.x as i32,
            z: pos.y as i32
        }
    }

    pub fn chunk_to_rect(&self, pos: ChunkPos) -> Rect {
        Rect {
            x: (pos.x as f32) * self.chunk_size, y: (pos.z as f32) * self.chunk_size,
            w: self.chunk_size, h: self.chunk_size
        }
    }

    pub fn chunk_rect_to_rect(&self, top_left: ChunkPos, bottom_right: ChunkPos) -> Rect {
        let width = (bottom_right.x - top_left.x + 1) as f32;
        let height = (bottom_right.z - top_left.z + 1) as f32;
        Rect {
            x: (top_left.x as f32) * self.chunk_size, y: (top_left.z as f32) * self.chunk_size,
            w: width * self.chunk_size, h: height * self.chunk_size
        }
    }
}
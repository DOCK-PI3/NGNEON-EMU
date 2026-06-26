/// Sistema de video NeoGeo: decodifica y genera el framebuffer
use std::sync::OnceLock;

pub const SCREEN_WIDTH: usize = 320;
pub const SCREEN_HEIGHT: usize = 224;
pub const SPRITE_TILE_WIDTH: usize = 16;
pub const SPRITE_TILE_HEIGHT: usize = 16;
const FIX_TILE_WIDTH: usize = 8;
const FIX_TILE_HEIGHT: usize = 8;
const BYTES_PER_FIX_TILE: usize = 32;
pub const BYTES_PER_TILE: usize = 128;
const SCB1_START: u16 = 0x0000;
const SCB1_WORDS_PER_SPRITE: u16 = 64;
const SCB2_START: u16 = 0x8000;
const SCB3_START: u16 = 0x8200;
const SCB4_START: u16 = 0x8400;
const SPRITES_PER_SCANLINE_LIMIT: usize = 96;
const FIX_MAP_START: u16 = 0x7000;
const FIX_MAP_COLUMNS: usize = 40;
const FIX_MAP_ROWS: usize = 32;
const FIX_HIDDEN_TOP_ROWS: usize = 2;
const FIX_VISIBLE_ROWS: usize = SCREEN_HEIGHT / FIX_TILE_HEIGHT;
const MAX_SPRITES: usize = 384;
const FIRST_RENDERABLE_SPRITE: usize = 1;
const SPRITE_PARSE_END: usize = 382;
const RENDERABLE_SPRITE_COUNT: usize = SPRITE_PARSE_END - FIRST_RENDERABLE_SPRITE;
const ALL_RENDERABLE_SPRITES: [u16; RENDERABLE_SPRITE_COUNT] = all_renderable_sprites();
const DIAGNOSTIC_PALETTE: [u32; 16] = [
    0xFF000000, 0xFF00F0FF, 0xFFFF2A8A, 0xFFFFD166, 0xFF9BFF6A, 0xFF6A7DFF, 0xFFFFFFFF, 0xFF606060,
    0xFF003040, 0xFF007080, 0xFF7A0050, 0xFF806000, 0xFF2D8A34, 0xFF272E88, 0xFFCCCCCC, 0xFF202020,
];
type PaletteLuts = ([u8; 64], [u8; 64]);
static PALETTE_LUTS: OnceLock<PaletteLuts> = OnceLock::new();

const fn all_renderable_sprites() -> [u16; RENDERABLE_SPRITE_COUNT] {
    let mut sprites = [0; RENDERABLE_SPRITE_COUNT];
    let mut i = 0;
    while i < RENDERABLE_SPRITE_COUNT {
        sprites[i] = (FIRST_RENDERABLE_SPRITE + i) as u16;
        i += 1;
    }
    sprites
}

const HSHRINK_LUT: [[bool; SPRITE_TILE_WIDTH]; 16] = [
    [
        false, false, false, false, false, false, false, false, true, false, false, false, false,
        false, false, false,
    ],
    [
        false, false, false, false, true, false, false, false, true, false, false, false, false,
        false, false, false,
    ],
    [
        false, false, false, false, true, false, false, false, true, false, false, false, true,
        false, false, false,
    ],
    [
        false, false, true, false, true, false, false, false, true, false, false, false, true,
        false, false, false,
    ],
    [
        false, false, true, false, true, false, false, false, true, false, false, false, true,
        false, true, false,
    ],
    [
        false, false, true, false, true, false, true, false, true, false, false, false, true,
        false, true, false,
    ],
    [
        false, false, true, false, true, false, true, false, true, false, true, false, true, false,
        true, false,
    ],
    [
        true, false, true, false, true, false, true, false, true, false, true, false, true, false,
        true, false,
    ],
    [
        true, false, true, false, true, false, true, false, true, true, true, false, true, false,
        true, false,
    ],
    [
        true, false, true, true, true, false, true, false, true, true, true, false, true, false,
        true, false,
    ],
    [
        true, false, true, true, true, false, true, false, true, true, true, false, true, false,
        true, true,
    ],
    [
        true, false, true, true, true, false, true, true, true, true, true, false, true, false,
        true, true,
    ],
    [
        true, false, true, true, true, false, true, true, true, true, true, false, true, true,
        true, true,
    ],
    [
        true, true, true, true, true, false, true, true, true, true, true, false, true, true, true,
        true,
    ],
    [
        true, true, true, true, true, false, true, true, true, true, true, true, true, true, true,
        true,
    ],
    [
        true, true, true, true, true, true, true, true, true, true, true, true, true, true, true,
        true,
    ],
];

pub struct Video {
    pub framebuffer: Vec<u32>, // ARGB8888, tamaño 320x224
    pub width: usize,
    pub height: usize,
    sprite_line_buffers: [[u16; SCREEN_WIDTH]; 2],
    sprite_line_buffer_active: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VramDebugStats {
    pub sprites_with_height: usize,
    pub generated_visible_sprites: usize,
    pub generated_sprite_scanlines: usize,
    pub generated_max_sprites_per_scanline: usize,
    pub generated_overflow_scanlines: usize,
    pub sprites_with_horizontal_shrink: usize,
    pub sprites_with_vertical_shrink: usize,
    pub visible_fix_tiles: usize,
    pub drawable_fix_tiles: usize,
    pub unique_fix_tiles: usize,
    pub fix_opaque_pixels: usize,
    pub initialized_palette_banks: usize,
}

#[derive(Debug, Clone, Copy)]
struct SpriteDrawParams {
    x: isize,
    y: isize,
    h_flip: bool,
    v_flip: bool,
    transparent_zero: bool,
    width: usize,
    height: usize,
}

#[derive(Debug, Clone, Copy)]
struct SpriteGroupState {
    y_position: usize,
    height_tiles: usize,
    vertical_shrink: u8,
    next_x: usize,
}

#[derive(Debug, Clone, Copy, Default)]
struct GeneratedSpriteStats {
    visible_sprites: usize,
    scanlines_with_sprites: usize,
    max_sprites_per_scanline: usize,
    overflow_scanlines: usize,
}

#[derive(Debug, Clone, Copy, Default)]
struct FixDebugStats {
    visible_tiles: usize,
    drawable_tiles: usize,
    unique_tiles: usize,
    opaque_pixels: usize,
}

impl Video {
    /// Clear the framebuffer (public — called by the emulation loop at frame start).
    pub fn clear_framebuffer(&mut self) {
        self.clear();
    }

    /// Reset the two hardware-like sprite line buffers.
    pub fn reset_sprite_line_buffers(&mut self) {
        self.sprite_line_buffers = [[0; SCREEN_WIDTH]; 2];
        self.sprite_line_buffer_active = 0;
    }

    /// Draw backdrop + the precomputed sprite line, then clear that buffer.
    pub fn present_buffered_sprites_scanline(
        &mut self,
        memory: &crate::memory::Memory,
        scanline: usize,
    ) {
        if scanline >= self.height {
            return;
        }

        let backdrop = backdrop_color(memory);
        let buffer_index = self.sprite_line_buffer_active;
        let fb_base = scanline * self.width;
        for x in 0..SCREEN_WIDTH {
            let palette_index = self.sprite_line_buffers[buffer_index][x];
            self.framebuffer[fb_base + x] = if palette_index == 0 {
                backdrop
            } else {
                palette_index_color(memory, palette_index)
            };
            self.sprite_line_buffers[buffer_index][x] = 0;
        }
    }

    /// Calculate a sprite line into the active buffer, then flip buffers.
    pub fn calculate_buffered_sprites_scanline_all(
        &mut self,
        memory: &crate::memory::Memory,
        scanline: usize,
    ) -> usize {
        if memory.crom.is_empty() {
            return 0;
        }

        let buffer_index = self.sprite_line_buffer_active;
        self.sprite_line_buffers[buffer_index].fill(0);
        let drawn = if scanline < self.height {
            render_scanline_sprites_to_line_buffer(self, memory, scanline, &ALL_RENDERABLE_SPRITES)
        } else {
            0
        };
        self.sprite_line_buffer_active ^= 1;
        drawn
    }

    /// Render all sprites for a single visible scanline, matching Geolith's
    /// `geo_lspc_sprcalc()` walk over sprite indices 1..381.
    pub fn render_sprites_scanline_all(&mut self, memory: &crate::memory::Memory, scanline: usize) {
        if memory.crom.is_empty() || scanline >= self.height {
            return;
        }

        render_scanline_sprites(self, memory, scanline, &ALL_RENDERABLE_SPRITES);
    }

    /// Render a single scanline of the fix layer into the framebuffer.
    ///
    /// Called during the per-scanline loop; only renders fix tiles that
    /// intersect the given scanline.
    pub fn render_fix_scanline(&mut self, memory: &crate::memory::Memory, scanline: usize) {
        let fix_rom = active_fix_rom(memory);
        if fix_rom.is_empty() || scanline >= self.height {
            return;
        }

        let row = scanline / FIX_TILE_HEIGHT;
        if row >= FIX_VISIBLE_ROWS {
            return;
        }

        let in_tile_y = scanline % FIX_TILE_HEIGHT;
        let map_row = row + FIX_HIDDEN_TOP_ROWS;
        let palette_base = active_palette_base(memory);

        for col in 0..FIX_MAP_COLUMNS {
            let map_address = fix_map_address(col, map_row);
            let entry = read_vram_word(memory, map_address);
            if entry == 0 || skip_fix_entry(memory, entry) {
                continue;
            }
            let tile_index = fix_tile_index(memory, entry, map_row, col);
            let palette_bank = ((entry >> 12) & 0x000F) as usize;

            let Some(tile) = decode_fix_tile(fix_rom, tile_index) else {
                continue;
            };

            let palette = decode_palette_bank(
                &memory.palette_ram,
                palette_base,
                palette_bank,
                memory.palette_shadow,
            );
            let fb_base = scanline * self.width + col * FIX_TILE_WIDTH;
            let tile_base = in_tile_y * FIX_TILE_WIDTH;

            for tile_x in 0..FIX_TILE_WIDTH {
                let color_index = tile[tile_base + tile_x] as usize;
                // Fix layer: color 0 is transparent (matches Geolith and
                // NeoGeo hardware behavior). Allows sprites behind
                // the fix layer to show through.
                if color_index == 0 {
                    continue;
                }

                let fb_x = col * FIX_TILE_WIDTH + tile_x;
                if fb_x < self.width {
                    self.framebuffer[fb_base + tile_x] = palette[color_index];
                }
            }
        }
    }

    /// Renderiza un frame de demo: píxel blanco movido por el usuario
    pub fn render_frame_demo(&mut self, memory: &crate::memory::Memory, px: usize, py: usize) {
        self.clear();

        // ── Si el display está deshabilitado, no renderizar ──
        if !memory.display_enabled {
            return;
        }

        for y in (0..self.height).step_by(16) {
            for x in 0..self.width {
                self.framebuffer[y * self.width + x] = 0xFF003040;
            }
        }
        for x in (0..self.width).step_by(16) {
            for y in 0..self.height {
                self.framebuffer[y * self.width + x] = 0xFF003040;
            }
        }

        let ship_color = 0xFF00F0FF;
        let core_color = 0xFFFF2A8A;
        for dy in 0..9 {
            for dx in 0..13 {
                let x = px.saturating_add(dx).saturating_sub(6);
                let y = py.saturating_add(dy).saturating_sub(4);
                if x >= self.width || y >= self.height {
                    continue;
                }

                let in_diamond = dx.abs_diff(6) + dy.abs_diff(4) <= 6;
                if in_diamond {
                    let idx = y * self.width + x;
                    self.framebuffer[idx] = if dx == 6 && dy == 4 {
                        core_color
                    } else {
                        ship_color
                    };
                }
            }
        }
    }

    /// Inicializa el sistema de video
    pub fn new() -> Self {
        Self {
            framebuffer: vec![0; SCREEN_WIDTH * SCREEN_HEIGHT],
            width: SCREEN_WIDTH,
            height: SCREEN_HEIGHT,
            sprite_line_buffers: [[0; SCREEN_WIDTH]; 2],
            sprite_line_buffer_active: 0,
        }
    }

    /// Limpia el framebuffer (pantalla negra)
    pub fn clear(&mut self) {
        self.clear_with_color(0xFF000000);
    }

    fn clear_with_color(&mut self, color: u32) {
        self.framebuffer.fill(color);
    }

    /// Renderiza tiles 16x16 de CROM en orden de índice, como el LSPC.
    pub fn render_frame(&mut self, memory: &crate::memory::Memory) {
        // ── Si el display está deshabilitado (POUTPUT bit 0 = 0), no renderizar ──
        if !memory.display_enabled {
            self.clear();
            return;
        }
        self.clear_with_color(backdrop_color(memory));
        self.reset_sprite_line_buffers();

        // Mirror the live Geolith-like scanline path even when rendering a
        // whole frame at once (pause, diagnostics, small-budget tests).
        let mut drawn_sprites = 0;
        drawn_sprites += self.calculate_buffered_sprites_scanline_all(memory, 0);
        drawn_sprites += self.calculate_buffered_sprites_scanline_all(memory, 1);
        for scanline in 0..self.height {
            self.present_buffered_sprites_scanline(memory, scanline);
            let future_scanline = scanline + 2;
            drawn_sprites += self.calculate_buffered_sprites_scanline_all(memory, future_scanline);
            self.render_fix_scanline(memory, scanline);
        }

        if drawn_sprites == 0 {
            self.clear_with_color(backdrop_color(memory));
            render_crom_diagnostic_matrix(self, memory);
            render_fix_layer(self, memory);
        }
    }
}

/// Render one scanline's worth of sprites from the given sprite indices.
///
/// This is the canonical single-scanline sprite renderer.  The full-frame
/// variant `render_vram_sprites_from_list` calls this function in a loop
/// so that sprite logic lives in exactly one place.
///
/// Returns the number of sprites drawn on this scanline.
fn render_scanline_sprites(
    video: &mut Video,
    memory: &crate::memory::Memory,
    scanline: usize,
    sprite_indices: &[u16],
) -> usize {
    if memory.crom.is_empty() || sprite_indices.is_empty() || scanline >= video.height {
        return 0;
    }

    let mut in_list = [false; MAX_SPRITES];
    for &sprite in sprite_indices {
        let index = sprite as usize;
        if index < MAX_SPRITES {
            in_list[index] = true;
        }
    }

    let mut matched_sprites = 0;
    let mut drawn = 0;
    let mut group: Option<SpriteGroupState> = None;

    for sprite in FIRST_RENDERABLE_SPRITE..SPRITE_PARSE_END {
        let sprite = sprite as u16;
        let in_active_list = in_list[sprite as usize];

        let y_control = read_vram_word(memory, scb3_addr(sprite));
        let is_sticky = y_control & 0x0040 != 0;
        let scb2 = read_vram_word(memory, scb2_addr(sprite));
        let shrink = shrink_from_scb2(scb2);

        let (x, y_position, height_tiles, vertical_shrink) = if is_sticky {
            let Some(ref group) = group else {
                continue;
            };
            (
                group.next_x,
                group.y_position,
                group.height_tiles,
                group.vertical_shrink,
            )
        } else {
            let height_tiles = sprite_height_tiles(y_control);
            let x_control = read_vram_word(memory, scb4_addr(sprite));
            let x = sprite_position_9bit(x_control);
            let y_position = sprite_position_9bit(y_control);
            (x, y_position, height_tiles, shrink.vertical)
        };

        let tile_width = shrink.horizontal_width;

        // Always update group state for sticky chains, even for sprites
        // not in the active list (they still affect following sticky sprites).
        group = Some(SpriteGroupState {
            y_position,
            height_tiles,
            vertical_shrink,
            next_x: (x + tile_width) & 0x01FF,
        });

        if height_tiles == 0 {
            continue;
        }

        if !in_active_list {
            continue;
        }

        if matched_sprites == SPRITES_PER_SCANLINE_LIMIT {
            break;
        }

        let sprite_row = sprite_row_for_scanline(scanline, y_position);
        if sprite_row < sprite_window_height(height_tiles) {
            matched_sprites += 1;
            drawn += draw_sprite_scanline(
                video,
                memory,
                sprite,
                scanline,
                SpriteDrawParams {
                    x: x as isize,
                    y: 0,
                    h_flip: false,
                    v_flip: false,
                    transparent_zero: true,
                    width: tile_width,
                    height: SPRITE_TILE_HEIGHT,
                },
                sprite_row,
                vertical_shrink,
                height_tiles,
            );
        }
    }

    drawn
}

fn render_scanline_sprites_to_line_buffer(
    video: &mut Video,
    memory: &crate::memory::Memory,
    scanline: usize,
    sprite_indices: &[u16],
) -> usize {
    if memory.crom.is_empty() || sprite_indices.is_empty() || scanline >= video.height {
        return 0;
    }

    let mut in_list = [false; MAX_SPRITES];
    for &sprite in sprite_indices {
        let index = sprite as usize;
        if index < MAX_SPRITES {
            in_list[index] = true;
        }
    }

    let mut matched_sprites = 0;
    let mut drawn = 0;
    let mut group: Option<SpriteGroupState> = None;

    for sprite in FIRST_RENDERABLE_SPRITE..SPRITE_PARSE_END {
        let sprite = sprite as u16;
        let in_active_list = in_list[sprite as usize];

        let y_control = read_vram_word(memory, scb3_addr(sprite));
        let is_sticky = y_control & 0x0040 != 0;
        let scb2 = read_vram_word(memory, scb2_addr(sprite));
        let shrink = shrink_from_scb2(scb2);

        let (x, y_position, height_tiles, vertical_shrink) = if is_sticky {
            let Some(ref group) = group else {
                continue;
            };
            (
                group.next_x,
                group.y_position,
                group.height_tiles,
                group.vertical_shrink,
            )
        } else {
            let height_tiles = sprite_height_tiles(y_control);
            let x_control = read_vram_word(memory, scb4_addr(sprite));
            let x = sprite_position_9bit(x_control);
            let y_position = sprite_position_9bit(y_control);
            (x, y_position, height_tiles, shrink.vertical)
        };

        let tile_width = shrink.horizontal_width;
        group = Some(SpriteGroupState {
            y_position,
            height_tiles,
            vertical_shrink,
            next_x: (x + tile_width) & 0x01FF,
        });

        if height_tiles == 0 || !in_active_list {
            continue;
        }

        if matched_sprites == SPRITES_PER_SCANLINE_LIMIT {
            break;
        }

        let sprite_row = sprite_row_for_scanline(scanline, y_position);
        if sprite_row < sprite_window_height(height_tiles) {
            matched_sprites += 1;
            drawn += draw_sprite_scanline_to_line_buffer(
                video,
                memory,
                sprite,
                SpriteDrawParams {
                    x: x as isize,
                    y: 0,
                    h_flip: false,
                    v_flip: false,
                    transparent_zero: true,
                    width: tile_width,
                    height: SPRITE_TILE_HEIGHT,
                },
                sprite_row,
                vertical_shrink,
                height_tiles,
            );
        }
    }

    drawn
}

#[allow(clippy::too_many_arguments)]
fn draw_sprite_scanline(
    video: &mut Video,
    memory: &crate::memory::Memory,
    sprite: u16,
    scanline: usize,
    params: SpriteDrawParams,
    sprite_row: usize,
    vertical_shrink: u8,
    height_tiles: usize,
) -> usize {
    let source_y = sprite_source_y(memory, sprite_row, height_tiles, vertical_shrink);

    let scb1 = SCB1_START + sprite * SCB1_WORDS_PER_SPRITE;
    let tile_map_offset = sprite_tile_map_offset(source_y);
    let tile_lsb = read_vram_word(memory, scb1 + tile_map_offset);
    let attr = read_vram_word(memory, scb1 + tile_map_offset + 1);
    let tile_index = tile_index_from_scb1(tile_lsb, attr, memory.auto_animation_counter());
    let Some(tile) = decode_sprite_tile(&memory.crom, tile_index, memory.lspc_rom_size) else {
        return 0;
    };

    let palette_bank = ((attr >> 8) & 0xFF) as usize;
    let palette_base = active_palette_base(memory);
    let palette = decode_palette_bank(
        &memory.palette_ram,
        palette_base,
        palette_bank,
        memory.palette_shadow,
    );
    draw_sprite_tile_scanline(
        video,
        &tile,
        &palette,
        SpriteDrawParams {
            h_flip: attr & 0x0001 != 0,
            v_flip: attr & 0x0002 != 0,
            ..params
        },
        scanline,
        source_y % SPRITE_TILE_HEIGHT,
    )
}

fn draw_sprite_scanline_to_line_buffer(
    video: &mut Video,
    memory: &crate::memory::Memory,
    sprite: u16,
    params: SpriteDrawParams,
    sprite_row: usize,
    vertical_shrink: u8,
    height_tiles: usize,
) -> usize {
    let source_y = sprite_source_y(memory, sprite_row, height_tiles, vertical_shrink);

    let scb1 = SCB1_START + sprite * SCB1_WORDS_PER_SPRITE;
    let tile_map_offset = sprite_tile_map_offset(source_y);
    let tile_lsb = read_vram_word(memory, scb1 + tile_map_offset);
    let attr = read_vram_word(memory, scb1 + tile_map_offset + 1);
    let tile_index = tile_index_from_scb1(tile_lsb, attr, memory.auto_animation_counter());
    let Some(tile) = decode_sprite_tile(&memory.crom, tile_index, memory.lspc_rom_size) else {
        return 0;
    };

    let palette_bank = ((attr >> 8) & 0xFF) as usize;
    let palette_word_base = active_palette_base(memory) / 2 + palette_bank * 16;
    draw_sprite_tile_scanline_to_line_buffer(
        video,
        &tile,
        palette_word_base,
        SpriteDrawParams {
            h_flip: attr & 0x0001 != 0,
            v_flip: attr & 0x0002 != 0,
            ..params
        },
        source_y % SPRITE_TILE_HEIGHT,
    )
}

fn render_crom_diagnostic_matrix(video: &mut Video, memory: &crate::memory::Memory) {
    let crom = &memory.crom;
    if crom.is_empty() {
        return;
    }

    let tiles_x = video.width / SPRITE_TILE_WIDTH;
    let tiles_y = video.height / SPRITE_TILE_HEIGHT;
    let palette_base = active_palette_base(memory);
    let palette = if palette_bank_initialized(&memory.palette_ram, palette_base, 0) {
        decode_palette_bank(&memory.palette_ram, palette_base, 0, memory.palette_shadow)
    } else {
        DIAGNOSTIC_PALETTE
    };

    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let tile_idx = ty * tiles_x + tx;
            let Some(tile) = decode_sprite_tile(crom, tile_idx, memory.lspc_rom_size) else {
                continue;
            };
            draw_sprite_tile(
                video,
                &tile,
                &palette,
                SpriteDrawParams {
                    x: (tx * SPRITE_TILE_WIDTH) as isize,
                    y: (ty * SPRITE_TILE_HEIGHT) as isize,
                    h_flip: false,
                    v_flip: false,
                    transparent_zero: false,
                    width: SPRITE_TILE_WIDTH,
                    height: SPRITE_TILE_HEIGHT,
                },
            );
        }
    }
}

fn render_fix_layer(video: &mut Video, memory: &crate::memory::Memory) -> usize {
    let fix_rom = active_fix_rom(memory);
    if fix_rom.is_empty() {
        return 0;
    }

    let fix_stats = fix_debug_stats(memory, fix_rom);
    if fix_stats.drawable_tiles == 0 {
        return 0;
    }

    let palette_base = active_palette_base(memory);
    let mut pixels = 0;
    for row in 0..FIX_VISIBLE_ROWS {
        let map_row = row + FIX_HIDDEN_TOP_ROWS;
        for col in 0..FIX_MAP_COLUMNS {
            let map_address = fix_map_address(col, map_row);
            let entry = read_vram_word(memory, map_address);
            if entry == 0 || skip_fix_entry(memory, entry) {
                continue;
            }
            let base_tile_index = fix_tile_index(memory, entry, map_row, col);
            let palette_bank = ((entry >> 12) & 0x000F) as usize;

            let Some(tile) = decode_fix_tile(fix_rom, base_tile_index) else {
                continue;
            };
            let palette = decode_palette_bank(
                &memory.palette_ram,
                palette_base,
                palette_bank,
                memory.palette_shadow,
            );
            pixels += draw_fix_tile(
                video,
                &tile,
                &palette,
                col * FIX_TILE_WIDTH,
                row * FIX_TILE_HEIGHT,
            );
        }
    }

    pixels
}

fn fix_tile_index(
    memory: &crate::memory::Memory,
    entry: u16,
    visible_row: usize,
    col: usize,
) -> usize {
    let base = (entry & 0x0FFF) as usize;
    match memory.fix_bankswitch {
        crate::memory::FixBankSwitch::None => base,
        crate::memory::FixBankSwitch::Line => base + fix_line_bank(memory, visible_row) * 0x1000,
        crate::memory::FixBankSwitch::Tile => {
            base + fix_tile_bank(memory, visible_row, col) * 0x1000
        }
    }
}

fn skip_fix_entry(memory: &crate::memory::Memory, entry: u16) -> bool {
    // Zed Blade (NGH 0x076) leaves a couple of palette-F FIX entries as screen
    // fillers. On real hardware and reference emulators these do not appear
    // over the game scene, but if rendered literally they produce the repeated
    // white/dark pattern reported in Zed Blade backgrounds.
    memory.rom_ngh == Some(0x0076) && matches!(entry, 0x00FF | 0xF020 | 0xF0FF)
}

/// Tile-based fix bankswitching: bank offset is computed from
/// VRAM[0x7500 + ((trow-1)&0x1f) + (32*(x/6))], extracting
/// the 2-bit bank value for the tile's column position within its group.
///
/// Mirrors Geolith's geo_lspc_fixline_tile() logic exactly:
///   fixline = neogeo_ram16[0x7500 + ((trow - 1) & 0x1f) + (32 * (tx / 6))]
///   fixline >>= (5 - (tx % 6)) * 2
///   fixbank = ~fixline & 0x03
///
/// The `visible_row` parameter is the hardware map row (0..31),
/// which includes FIX_HIDDEN_TOP_ROWS (2). The first rendered row in
/// NGNEON is hardware map row 2, so this matches Geolith's active
/// `trow - 1` lookup after the visible crop.
///
/// Used by KOF2000, Matrimelee, SVC, KOF2003 (NGH 0x257, 0x266, 0x269, 0x271).
fn fix_tile_bank(memory: &crate::memory::Memory, visible_row: usize, col: usize) -> usize {
    let row_offset = (visible_row.wrapping_sub(1)) & 0x1f;
    let group_offset = 32 * (col / 6);
    let vram_addr = 0x7500u16 + (row_offset + group_offset) as u16;
    let value = read_vram_word(memory, vram_addr);
    // Extract the 2-bit bank value: for tile within group of 6,
    // shift right by (5 - (col % 6)) * 2 bits, mask with 0x03
    let shift = (5 - (col % 6)) * 2;
    let bank_bits = ((value >> shift) & 0x0003) as usize;
    // Complement the bits (~) per Geolith
    (!bank_bits) & 0x03
}

fn fix_line_bank(memory: &crate::memory::Memory, visible_row: usize) -> usize {
    let mut bank = 3usize;
    let target_row = visible_row.wrapping_sub(FIX_HIDDEN_TOP_ROWS) & 0x1f;

    for row in 0..FIX_MAP_ROWS {
        let offset = row as u16;
        let bank_select = read_vram_word(memory, 0x7500 + offset);
        let bank_value = read_vram_word(memory, 0x7580 + offset);
        if bank_select == 0x0200 && (bank_value & 0xFF00) == 0xFF00 {
            bank = ((!bank_value) & 0x0003) as usize;
        }

        if row == target_row {
            return bank;
        }
    }

    bank
}

fn draw_fix_tile(
    video: &mut Video,
    tile: &[u8; FIX_TILE_WIDTH * FIX_TILE_HEIGHT],
    palette: &[u32; 16],
    x: usize,
    y: usize,
) -> usize {
    let mut pixels = 0;
    for tile_y in 0..FIX_TILE_HEIGHT {
        for tile_x in 0..FIX_TILE_WIDTH {
            let color_index = tile[tile_y * FIX_TILE_WIDTH + tile_x] as usize;
            // Color 0 is transparent for fix tiles (matches Geolith and
            // NeoGeo hardware behavior). Allows sprites behind the fix
            // layer to show through.
            if color_index == 0 {
                continue;
            }

            let fb_x = x + tile_x;
            let fb_y = y + tile_y;
            if fb_x >= video.width || fb_y >= video.height {
                continue;
            }

            video.framebuffer[fb_y * video.width + fb_x] = palette[color_index];
            pixels += 1;
        }
    }

    pixels
}

fn draw_sprite_tile(
    video: &mut Video,
    tile: &[u8; 256],
    palette: &[u32; 16],
    params: SpriteDrawParams,
) -> usize {
    if params.width == 0 || params.height == 0 {
        return 0;
    }

    let mut pixels = 0;
    for dest_y in 0..params.height {
        for dest_x in 0..params.width {
            let unflipped_x = dest_x * SPRITE_TILE_WIDTH / params.width;
            let unflipped_y = dest_y * SPRITE_TILE_HEIGHT / params.height;
            let src_x = if params.h_flip {
                SPRITE_TILE_WIDTH - 1 - unflipped_x
            } else {
                unflipped_x
            };
            let src_y = if params.v_flip {
                SPRITE_TILE_HEIGHT - 1 - unflipped_y
            } else {
                unflipped_y
            };
            let color_index = tile[src_y * SPRITE_TILE_WIDTH + src_x] as usize;
            if params.transparent_zero && color_index == 0 {
                continue;
            }

            let fb_x = params.x + dest_x as isize;
            let fb_y = params.y + dest_y as isize;
            if fb_x < 0 || fb_y < 0 || fb_x >= video.width as isize || fb_y >= video.height as isize
            {
                continue;
            }

            video.framebuffer[fb_y as usize * video.width + fb_x as usize] = palette[color_index];
            pixels += 1;
        }
    }

    pixels
}

fn draw_sprite_tile_scanline(
    video: &mut Video,
    tile: &[u8; 256],
    palette: &[u32; 16],
    params: SpriteDrawParams,
    scanline: usize,
    tile_dest_y: usize,
) -> usize {
    if params.width == 0 || params.height == 0 || scanline >= video.height {
        return 0;
    }

    let unflipped_y = tile_dest_y * SPRITE_TILE_HEIGHT / params.height;
    let src_y = if params.v_flip {
        SPRITE_TILE_HEIGHT - 1 - unflipped_y
    } else {
        unflipped_y
    };

    let hshrink = params.width.saturating_sub(1).min(HSHRINK_LUT.len() - 1);
    let mut pixels = 0;
    let mut dest_x = 0usize;
    for (source_column, visible) in HSHRINK_LUT[hshrink].iter().copied().enumerate() {
        if !visible {
            continue;
        }

        let src_x = if params.h_flip {
            SPRITE_TILE_WIDTH - 1 - source_column
        } else {
            source_column
        };
        let color_index = tile[src_y * SPRITE_TILE_WIDTH + src_x] as usize;
        if params.transparent_zero && color_index == 0 {
            dest_x += 1;
            continue;
        }

        let fb_x = ((params.x + dest_x as isize) & 0x01FF) as usize;
        if fb_x >= video.width {
            dest_x += 1;
            continue;
        }

        video.framebuffer[scanline * video.width + fb_x] = palette[color_index];
        pixels += 1;
        dest_x += 1;
    }

    pixels
}

fn draw_sprite_tile_scanline_to_line_buffer(
    video: &mut Video,
    tile: &[u8; 256],
    palette_word_base: usize,
    params: SpriteDrawParams,
    tile_dest_y: usize,
) -> usize {
    if params.width == 0 || params.height == 0 {
        return 0;
    }

    let unflipped_y = tile_dest_y * SPRITE_TILE_HEIGHT / params.height;
    let src_y = if params.v_flip {
        SPRITE_TILE_HEIGHT - 1 - unflipped_y
    } else {
        unflipped_y
    };

    let hshrink = params.width.saturating_sub(1).min(HSHRINK_LUT.len() - 1);
    let buffer_index = video.sprite_line_buffer_active;
    let mut pixels = 0;
    let mut dest_x = 0usize;
    for (source_column, visible) in HSHRINK_LUT[hshrink].iter().copied().enumerate() {
        if !visible {
            continue;
        }

        let src_x = if params.h_flip {
            SPRITE_TILE_WIDTH - 1 - source_column
        } else {
            source_column
        };
        let color_index = tile[src_y * SPRITE_TILE_WIDTH + src_x] as usize;
        if params.transparent_zero && color_index == 0 {
            dest_x += 1;
            continue;
        }

        let fb_x = ((params.x + dest_x as isize) & 0x01FF) as usize;
        if fb_x < SCREEN_WIDTH {
            let palette_index = palette_word_base + color_index;
            if let Ok(palette_index) = u16::try_from(palette_index) {
                video.sprite_line_buffers[buffer_index][fb_x] = palette_index;
                pixels += 1;
            }
        }
        dest_x += 1;
    }

    pixels
}

/// Calcula la máscara C-ROM estilo Geolith.
///
/// Equivalente a `geo_calc_mask(32, crom_size / 128)` en Geolith:
/// devuelve `next_power_of_two(num_tiles) - 1`, que es el bitmask
/// necesario para que los índices de tile hagan wrap-around dentro
/// del espacio C-ROM disponible.
///
/// Ejemplo: con 640 tiles → next_power_of_two=1024 → mask=1023 (0x3FF).
/// Cualquier tile_index & 0x3FF da siempre un tile válido en [0, 1023].
pub fn calc_crom_mask(crom_size: usize) -> usize {
    if crom_size < BYTES_PER_TILE {
        return 0;
    }
    let tiles = crom_size / BYTES_PER_TILE;
    tiles.next_power_of_two() - 1
}

/// Convierte el valor del registro LSPC RomSize (0x3C000C) a una
/// máscara de tiles C-ROM, replicando la lógica de Geolith.
///
/// En el hardware NeoGeo, el registro RomSize codifica el tamaño
/// del espacio de direcciones C-ROM: cada unidad representa un
/// rango de 512KB, por lo que la máscara es `(1 << (rom_size + 12)) - 1`
/// donde 12 = log2(512KB / 128 bytes por tile) = log2(4096).
///
/// Ejemplos:
/// - rom_size = 0 (512KB)  → mask = (1 << 12) - 1 = 4095  (0xFFF)
/// - rom_size = 3 (4MB)    → mask = (1 << 15) - 1 = 32767 (0x7FFF)
/// - rom_size = 5 (16MB)   → mask = (1 << 17) - 1 = 131071 (0x1FFFF)
pub fn register_to_crom_mask(rom_size: u16) -> usize {
    let bits = ((rom_size as usize & 0x1F) + 12).min(usize::BITS as usize - 1);
    if bits >= usize::BITS as usize {
        return !0; // All bits set = wrap around entire address space
    }
    (1usize << bits) - 1
}

/// Verifica que el registro LSPC RomSize (0x3C000C) sea compatible
/// con la máscara C-ROM calculada a partir del tamaño real de datos.
///
/// Esto replica la verificación que Geolith hace en `geo_lspc_postload`:
/// después de cargar la ROM, comprueba que el valor escrito por el juego
/// en el registro RomSize coincida con el tamaño real del C-ROM.
///
/// Devuelve `true` si son compatibles, o si el registro aún no ha sido
/// inicializado (rom_size == 0). Imprime un warning en stderr si
/// detecta una discrepancia.
pub fn verify_crom_mask_register(memory: &crate::memory::Memory) -> bool {
    if memory.lspc_rom_size == 0 || memory.crom.is_empty() {
        return true; // Register not yet written by the game, can't verify
    }

    let data_mask = calc_crom_mask(memory.crom.len());
    let reg_mask = register_to_crom_mask(memory.lspc_rom_size);

    // The data mask should be >= the register mask. If the game programmed
    // a smaller window, it simply can't address all tiles. If it programmed
    // a larger window, the extra addresses wrap (which our crom_mask handles).
    if data_mask >= reg_mask {
        return true;
    }

    // Data mask < register mask: the game expects more C-ROM than we have.
    // This is a soft warning — the game may still work if it doesn't access
    // tiles beyond the actual C-ROM.
    eprintln!(
        "[WARN] LSPC RomSize mismatch: reg=0x{:04X} (mask 0x{:X}) but data crom_mask=0x{:X} ({:.1}MB)",
        memory.lspc_rom_size,
        reg_mask,
        data_mask,
        memory.crom.len() as f64 / (1024.0 * 1024.0),
    );
    false
}

pub fn decode_sprite_tile(
    crom: &[u8],
    tile_index: usize,
    _lspc_rom_size: u16,
) -> Option<[u8; 256]> {
    // Geolith computes crommask once from the real C-ROM size in
    // geo_lspc_postload(): geo_calc_mask(32, romdata->csz >> 7). Runtime
    // writes to 0x3C000C are IRQ acknowledgements in the games we run here;
    // using them as a sprite mask makes some games intermittently fetch the
    // wrong tile. Keep the third parameter only to avoid churn at call sites.
    let mask = calc_crom_mask(crom.len());
    let tile_count = crom.len() / BYTES_PER_TILE;
    if tile_count == 0 {
        return None;
    }
    let masked_index = tile_index & mask;
    let base = (masked_index % tile_count).checked_mul(BYTES_PER_TILE)?;
    let tile_data = crom.get(base..base + BYTES_PER_TILE)?;
    let mut pixels = [0u8; SPRITE_TILE_WIDTH * SPRITE_TILE_HEIGHT];

    // NeoGeo C-ROM tile format (matches Geolith):
    // 16×16 pixels, 4bpp, 128 bytes per tile.
    // The pixel data is stored right-to-left in two 64-byte halves. Geolith
    // compensates by reading the second half first when hflip is disabled
    // (`(((0x08 & p) ^ 0x08) << 3)`). Decode into normal display order here;
    // the draw step can then apply a regular 16-pixel horizontal flip.
    // Each half has 16 rows × 4 bytes. Cart C ROM bitplanes are read in
    // Geolith order [0, 2, 1, 3], producing palette bits [0, 1, 2, 3].
    // Bit order: bit 0 (LSB) = first displayed pixel in the 8-pixel half.
    for y in 0..SPRITE_TILE_HEIGHT {
        for x in 0..SPRITE_TILE_WIDTH {
            let half_base = if x < 8 { 64 } else { 0 };
            let row_base = half_base + y * 4;
            let bit = x & 7;
            let color = ((tile_data[row_base] >> bit) & 1)
                | (((tile_data[row_base + 2] >> bit) & 1) << 1)
                | (((tile_data[row_base + 1] >> bit) & 1) << 2)
                | (((tile_data[row_base + 3] >> bit) & 1) << 3);
            pixels[y * SPRITE_TILE_WIDTH + x] = color;
        }
    }

    Some(pixels)
}

pub fn decode_fix_tile(
    srom: &[u8],
    tile_index: usize,
) -> Option<[u8; FIX_TILE_WIDTH * FIX_TILE_HEIGHT]> {
    let base = tile_index.checked_mul(BYTES_PER_FIX_TILE)?;
    let tile_data = srom.get(base..base + BYTES_PER_FIX_TILE)?;
    let mut pixels = [0u8; FIX_TILE_WIDTH * FIX_TILE_HEIGHT];

    // NeoGeo fix tile format: 32 bytes per 8×8 tile, interleaved halves.
    // Matches Geolith's geo_lspc_fixline_default access pattern:
    //   byte[16+y] → cols 0-1, byte[24+y] → cols 2-3
    //   byte[ 0+y] → cols 4-5, byte[ 8+y] → cols 6-7
    // Geolith: low nibble = left pixel, high nibble = right pixel.
    //   dst[x]   = d & 0x0f         → left pixel
    //   dst[x+1] = (d >> 4) & 0x0f  → right pixel
    for y in 0..FIX_TILE_HEIGHT {
        let b0 = tile_data[16 + y]; // cols 0,1
        let b1 = tile_data[24 + y]; // cols 2,3
        let b2 = tile_data[y]; // cols 4,5
        let b3 = tile_data[8 + y]; // cols 6,7
        pixels[y * 8] = b0 & 0x0f;
        pixels[y * 8 + 1] = b0 >> 4;
        pixels[y * 8 + 2] = b1 & 0x0f;
        pixels[y * 8 + 3] = b1 >> 4;
        pixels[y * 8 + 4] = b2 & 0x0f;
        pixels[y * 8 + 5] = b2 >> 4;
        pixels[y * 8 + 6] = b3 & 0x0f;
        pixels[y * 8 + 7] = b3 >> 4;
    }

    Some(pixels)
}

fn decode_palette_bank(
    palette_ram: &[u8],
    active_base: usize,
    bank: usize,
    shadow: bool,
) -> [u32; 16] {
    let start = active_base.saturating_add(bank.saturating_mul(32));
    let Some(bank_data) = palette_ram.get(start..start + 32) else {
        return DIAGNOSTIC_PALETTE;
    };

    let mut palette = [0xFF000000; 16];
    for (i, color_slot) in palette.iter_mut().enumerate() {
        let base = i * 2;
        let color = u16::from_be_bytes([bank_data[base], bank_data[base + 1]]);
        *color_slot = decode_palette_color_with_shadow(color, shadow);
    }

    palette
}

fn palette_bank_initialized(palette_ram: &[u8], active_base: usize, bank: usize) -> bool {
    let start = active_base.saturating_add(bank.saturating_mul(32));
    palette_ram
        .get(start..start + 32)
        .is_some_and(|bank_data| bank_data.iter().any(|&byte| byte != 0))
}

fn active_palette_base(memory: &crate::memory::Memory) -> usize {
    memory.palette_bank as usize * crate::memory::PALETTE_RAM_BANK_SIZE
}

fn backdrop_color(memory: &crate::memory::Memory) -> u32 {
    let offset = active_palette_base(memory).saturating_add(4095 * 2);
    let Some(color_data) = memory.palette_ram.get(offset..offset + 2) else {
        return 0xFF000000;
    };

    decode_palette_color_with_shadow(
        u16::from_be_bytes([color_data[0], color_data[1]]),
        memory.palette_shadow,
    )
}

fn palette_index_color(memory: &crate::memory::Memory, palette_index: u16) -> u32 {
    let offset = (palette_index as usize).saturating_mul(2);
    let Some(color_data) = memory.palette_ram.get(offset..offset + 2) else {
        return 0xFF000000;
    };

    decode_palette_color_with_shadow(
        u16::from_be_bytes([color_data[0], color_data[1]]),
        memory.palette_shadow,
    )
}

fn active_fix_rom(memory: &crate::memory::Memory) -> &[u8] {
    if memory.use_cart_fix && !memory.dynamic_fix_rom.is_empty() {
        return &memory.dynamic_fix_rom;
    }
    if memory.use_cart_fix || memory.sfix.is_empty() {
        &memory.srom
    } else {
        &memory.sfix
    }
}

fn read_vram_word(memory: &crate::memory::Memory, address: u16) -> u16 {
    let offset = (address as usize * 2) % memory.vram.len();
    u16::from_be_bytes([
        memory.vram[offset],
        memory.vram[(offset + 1) % memory.vram.len()],
    ])
}

fn fix_map_address(col: usize, row: usize) -> u16 {
    // NeoGeo fix map is column-major: 40 columns × 32 rows.
    // Each entry is a 16-bit word at FIX_MAP_START + (col * FIX_MAP_ROWS + row).
    // This matches the LSPC hardware addressing: address = 0x7000 + (col << 5) + row.
    FIX_MAP_START + (col * FIX_MAP_ROWS + row) as u16
}

fn tile_index_from_scb1(tile_lsb: u16, attr: u16, auto_animation_counter: Option<u8>) -> usize {
    let tile_msb = ((attr >> 4) & 0x000F) as usize;
    let mut tile_index = (tile_msb << 16) | tile_lsb as usize;
    if let Some(counter) = auto_animation_counter {
        match (attr >> 2) & 0x0003 {
            1 => {
                tile_index &= !0x0003;
                tile_index |= (counter & 0x03) as usize;
            }
            2 | 3 => {
                tile_index &= !0x0007;
                tile_index |= (counter & 0x07) as usize;
            }
            _ => {}
        }
    }

    tile_index
}

fn scb2_addr(sprite: u16) -> u16 {
    SCB2_START + sprite
}

fn scb3_addr(sprite: u16) -> u16 {
    SCB3_START + sprite
}

fn scb4_addr(sprite: u16) -> u16 {
    SCB4_START + sprite
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SpriteShrink {
    horizontal_width: usize,
    vertical: u8,
}

fn shrink_from_scb2(scb2: u16) -> SpriteShrink {
    SpriteShrink {
        horizontal_width: (((scb2 >> 8) & 0x000F) + 1) as usize,
        vertical: scb2 as u8,
    }
}

fn sprite_height_tiles(y_control: u16) -> usize {
    (y_control & 0x003F) as usize
}

fn sprite_window_height(height_tiles: usize) -> usize {
    height_tiles * SPRITE_TILE_HEIGHT
}

fn sprite_tile_map_offset(source_y: usize) -> u16 {
    ((source_y >> 3) & 0x3E) as u16
}

fn shrunk_sprite_graphics_height(height_tiles: usize, vertical_shrink: u8) -> usize {
    ((sprite_window_height(height_tiles) * (vertical_shrink as usize + 1)).div_ceil(256)).max(1)
}

fn sprite_source_y(
    memory: &crate::memory::Memory,
    local_y: usize,
    height_tiles: usize,
    vertical_shrink: u8,
) -> usize {
    if let Some(source_y) =
        sprite_source_y_from_l0(&memory.zoom_rom, local_y, height_tiles, vertical_shrink)
    {
        return source_y;
    }

    let window_height = sprite_window_height(height_tiles);
    let graphics_height = shrunk_sprite_graphics_height(height_tiles, vertical_shrink);
    let graphics_y = local_y.min(graphics_height.saturating_sub(1));

    (graphics_y * window_height / graphics_height).min(window_height.saturating_sub(1))
}

fn sprite_source_y_from_l0(
    zoom_rom: &[u8],
    sprite_row: usize,
    height_tiles: usize,
    vertical_shrink: u8,
) -> Option<usize> {
    if zoom_rom.len() != crate::bios::ZOOM_ROM_SIZE || height_tiles == 0 {
        return None;
    }

    let mut invert = sprite_row > 0xFF;
    let mut zoom_row = sprite_row & 0xFF;
    if invert {
        zoom_row ^= 0xFF;
    }

    // Geolith special-cases raw size 33 by looping the zoom lookup border.
    // This matters for a few real games and prevents one-line vertical
    // cracks when a 33-tile sprite is shrunk.
    if height_tiles == 33 {
        let period = ((vertical_shrink as usize + 1) << 1).max(1);
        zoom_row %= period;
        if zoom_row > vertical_shrink as usize {
            zoom_row = period - 1 - zoom_row;
            invert = !invert;
        }
    }

    let table_offset = vertical_shrink as usize * 0x100 + zoom_row;
    let entry = *zoom_rom.get(table_offset)?;
    let mut source_y = entry as usize;
    if invert {
        source_y ^= 0x01FF;
    }

    Some(source_y)
}

fn sprite_position_9bit(control: u16) -> usize {
    ((control >> 7) & 0x01FF) as usize
}

fn sprite_row_for_scanline(scanline: usize, y_position: usize) -> usize {
    // NGNEON renders the 224 visible lines directly. Geolith renders into a
    // taller LSPC buffer and libretro exposes it from line crop_t + 16, so add
    // the hardware-visible top offset before applying the 9-bit sprite wrap.
    (scanline + 16 + y_position).wrapping_sub(0x200) & 0x01FF
}

#[cfg(test)]
fn decode_palette_color(color: u16) -> u32 {
    decode_palette_color_with_shadow(color, false)
}

fn decode_palette_color_with_shadow(color: u16, shadow: bool) -> u32 {
    // Geolith-compatible NeoGeo palette words:
    // D0 R1 G1 B1 R5 R4 R3 R2 G5 G4 G3 G2 B5 B4 B3 B2.
    // The dark bit is folded into bit 0 of each LUT index.
    let luts = palette_luts();
    let lut = if shadow { &luts.1 } else { &luts.0 };
    let r = (((color >> 6) & 0x3C) | ((color >> 13) & 0x02) | ((color >> 15) & 0x01)) as usize;
    let g = (((color >> 2) & 0x3C) | ((color >> 12) & 0x02) | ((color >> 15) & 0x01)) as usize;
    let b = (((color << 2) & 0x3C) | ((color >> 11) & 0x02) | ((color >> 15) & 0x01)) as usize;

    0xFF000000 | ((lut[r] as u32) << 16) | ((lut[g] as u32) << 8) | lut[b] as u32
}

fn palette_luts() -> &'static PaletteLuts {
    PALETTE_LUTS.get_or_init(generate_geolith_palette_luts)
}

fn generate_geolith_palette_luts() -> PaletteLuts {
    let resistance = [3900.0, 2200.0, 1000.0, 470.0, 220.0];
    let pd_dark = 8200.0;
    let pd_shadow = 150.0;

    let mut v_raw = [0.0; 32];
    for (i, out) in v_raw.iter_mut().enumerate() {
        let (r_to_vcc, r_to_gnd) = color_resistance(i, &resistance);
        *out = voltage_from_resistance(r_to_vcc, r_to_gnd);
    }

    let mut v_smooth = [0.0; 32];
    v_smooth[0] = v_raw[0];
    v_smooth[31] = v_raw[31];
    for i in 1..31 {
        v_smooth[i] = (v_raw[i - 1] * 1.6 + v_raw[i] + v_raw[i + 1] * 1.6) / 4.2;
    }

    let v_min = v_smooth[0];
    let v_max = v_smooth[31];
    let mut normal = [0; 64];
    let mut shadow = [0; 64];

    for (i, smooth) in v_smooth.iter().copied().enumerate() {
        let (r_to_vcc, r_to_gnd) = color_resistance(i, &resistance);
        let base = ((smooth - v_min) / (v_max - v_min)) * 255.0;

        let mut factor_dark = 1.0;
        let mut factor_shadow = 1.0;
        let mut factor_both = 1.0;

        if r_to_vcc > 0.0 {
            let v_normal = voltage_from_resistance(r_to_vcc, r_to_gnd);
            let r_gnd_dark = parallel_or_only(r_to_gnd, pd_dark);
            let r_gnd_shadow = parallel_or_only(r_to_gnd, pd_shadow);
            let combined = parallel(pd_dark, pd_shadow);
            let r_gnd_both = parallel_or_only(r_to_gnd, combined);
            let v_dark = r_gnd_dark / (r_to_vcc + r_gnd_dark);
            let v_shadow = r_gnd_shadow / (r_to_vcc + r_gnd_shadow);
            let v_both = r_gnd_both / (r_to_vcc + r_gnd_both);

            if v_normal > 0.0 {
                factor_dark = v_dark / v_normal;
                factor_shadow = v_shadow / v_normal;
                factor_both = v_both / v_normal;
            }
        }

        let light = i << 1;
        let dark = light | 1;
        normal[light] = rounded_u8(base);
        normal[dark] = rounded_u8(base * factor_dark);
        shadow[light] = rounded_u8(base * factor_shadow);
        shadow[dark] = rounded_u8(base * factor_both);
    }

    (normal, shadow)
}

fn color_resistance(color: usize, resistance: &[f64; 5]) -> (f64, f64) {
    let mut r_to_vcc = 0.0;
    let mut r_to_gnd = 0.0;

    for (bit, &r) in resistance.iter().enumerate() {
        if color & (1 << bit) != 0 {
            r_to_vcc = parallel_or_only(r_to_vcc, r);
        } else {
            r_to_gnd = parallel_or_only(r_to_gnd, r);
        }
    }

    (r_to_vcc, r_to_gnd)
}

fn voltage_from_resistance(r_to_vcc: f64, r_to_gnd: f64) -> f64 {
    if r_to_vcc == 0.0 {
        0.0
    } else if r_to_gnd == 0.0 {
        1.0
    } else {
        r_to_gnd / (r_to_vcc + r_to_gnd)
    }
}

fn parallel_or_only(current: f64, r: f64) -> f64 {
    if current == 0.0 {
        r
    } else {
        parallel(current, r)
    }
}

fn parallel(a: f64, b: f64) -> f64 {
    (a * b) / (a + b)
}

fn rounded_u8(value: f64) -> u8 {
    (value + 0.5).clamp(0.0, 255.0) as u8
}

impl Default for Video {
    fn default() -> Self {
        Self::new()
    }
}

pub fn debug_vram_stats(memory: &crate::memory::Memory) -> VramDebugStats {
    let sprites_with_height = (FIRST_RENDERABLE_SPRITE..SPRITE_PARSE_END)
        .filter(|sprite| {
            let y_control = read_vram_word(memory, scb3_addr(*sprite as u16));
            sprite_height_tiles(y_control) != 0
        })
        .count();

    let generated_sprite_stats = generated_sprite_stats(memory);
    let shrink_stats = shrink_debug_stats(memory);

    let fix_stats = fix_debug_stats(memory, active_fix_rom(memory));

    let initialized_palette_banks = (0..256)
        .filter(|palette_bank| {
            palette_bank_initialized(
                &memory.palette_ram,
                active_palette_base(memory),
                *palette_bank,
            )
        })
        .count();

    VramDebugStats {
        sprites_with_height,
        generated_visible_sprites: generated_sprite_stats.visible_sprites,
        generated_sprite_scanlines: generated_sprite_stats.scanlines_with_sprites,
        generated_max_sprites_per_scanline: generated_sprite_stats.max_sprites_per_scanline,
        generated_overflow_scanlines: generated_sprite_stats.overflow_scanlines,
        sprites_with_horizontal_shrink: shrink_stats.horizontal,
        sprites_with_vertical_shrink: shrink_stats.vertical,
        visible_fix_tiles: fix_stats.visible_tiles,
        drawable_fix_tiles: fix_stats.drawable_tiles,
        unique_fix_tiles: fix_stats.unique_tiles,
        fix_opaque_pixels: fix_stats.opaque_pixels,
        initialized_palette_banks,
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct ShrinkDebugStats {
    horizontal: usize,
    vertical: usize,
}

fn shrink_debug_stats(memory: &crate::memory::Memory) -> ShrinkDebugStats {
    let mut stats = ShrinkDebugStats::default();
    let mut group: Option<SpriteGroupState> = None;

    for sprite in FIRST_RENDERABLE_SPRITE..SPRITE_PARSE_END {
        let sprite = sprite as u16;
        let y_control = read_vram_word(memory, scb3_addr(sprite));
        let is_sticky = y_control & 0x0040 != 0;
        let scb2 = read_vram_word(memory, scb2_addr(sprite));
        let shrink = shrink_from_scb2(scb2);

        let (_height_tiles, vertical_shrink) = if is_sticky {
            let Some(group) = group else {
                continue;
            };
            (group.height_tiles, group.vertical_shrink)
        } else {
            let height_tiles = sprite_height_tiles(y_control);
            if height_tiles == 0 {
                group = None;
                continue;
            }

            let x_control = read_vram_word(memory, scb4_addr(sprite));
            let x = sprite_position_9bit(x_control);
            let y_position = sprite_position_9bit(y_control);
            group = Some(SpriteGroupState {
                y_position,
                height_tiles,
                vertical_shrink: shrink.vertical,
                next_x: (x + shrink.horizontal_width) & 0x01FF,
            });
            (height_tiles, shrink.vertical)
        };

        if shrink.horizontal_width != SPRITE_TILE_WIDTH {
            stats.horizontal += 1;
        }
        if vertical_shrink != 0xFF {
            stats.vertical += 1;
        }
    }

    stats
}

fn generated_sprite_stats(memory: &crate::memory::Memory) -> GeneratedSpriteStats {
    let mut counts = [0usize; SCREEN_HEIGHT];
    let mut visible_sprites = 0;
    let mut group: Option<SpriteGroupState> = None;

    for sprite in FIRST_RENDERABLE_SPRITE..SPRITE_PARSE_END {
        let sprite = sprite as u16;
        let y_control = read_vram_word(memory, scb3_addr(sprite));
        let is_sticky = y_control & 0x0040 != 0;
        let scb2 = read_vram_word(memory, scb2_addr(sprite));
        let shrink = shrink_from_scb2(scb2);

        let (y_position, height_tiles, _vertical_shrink) = if is_sticky {
            let Some(group) = group else {
                continue;
            };
            (group.y_position, group.height_tiles, group.vertical_shrink)
        } else {
            let height_tiles = sprite_height_tiles(y_control);
            if height_tiles == 0 {
                group = None;
                continue;
            }

            let x_control = read_vram_word(memory, scb4_addr(sprite));
            let x = sprite_position_9bit(x_control);
            let y_position = sprite_position_9bit(y_control);
            group = Some(SpriteGroupState {
                y_position,
                height_tiles,
                vertical_shrink: shrink.vertical,
                next_x: (x + shrink.horizontal_width) & 0x01FF,
            });
            (y_position, height_tiles, shrink.vertical)
        };

        let sprite_height = sprite_window_height(height_tiles.min(32));
        let mut visible = false;
        for (scanline, count) in counts.iter_mut().enumerate() {
            if sprite_row_for_scanline(scanline, y_position) < sprite_height {
                *count += 1;
                visible = true;
            }
        }
        visible_sprites += usize::from(visible);
    }

    let scanlines_with_sprites = counts.iter().filter(|count| **count != 0).count();
    let max_sprites_per_scanline = counts.iter().copied().max().unwrap_or(0);
    let overflow_scanlines = counts
        .iter()
        .filter(|count| **count > SPRITES_PER_SCANLINE_LIMIT)
        .count();

    GeneratedSpriteStats {
        visible_sprites,
        scanlines_with_sprites,
        max_sprites_per_scanline,
        overflow_scanlines,
    }
}

fn fix_debug_stats(memory: &crate::memory::Memory, fix_rom: &[u8]) -> FixDebugStats {
    if fix_rom.is_empty() {
        return FixDebugStats::default();
    }

    let mut stats = FixDebugStats::default();
    let mut seen_tiles = vec![false; fix_rom.len() / BYTES_PER_FIX_TILE];
    for row in 0..FIX_VISIBLE_ROWS {
        let map_row = row + FIX_HIDDEN_TOP_ROWS;
        for col in 0..FIX_MAP_COLUMNS {
            let map_address = fix_map_address(col, map_row);
            let entry = read_vram_word(memory, map_address);
            if entry == 0 || skip_fix_entry(memory, entry) {
                continue;
            }
            let tile_index = fix_tile_index(memory, entry, map_row, col);
            stats.visible_tiles += 1;
            if seen_tiles.get_mut(tile_index).is_some_and(|seen| {
                let was_seen = *seen;
                *seen = true;
                !was_seen
            }) {
                stats.unique_tiles += 1;
            }

            let Some(tile) = decode_fix_tile(fix_rom, tile_index) else {
                continue;
            };
            let opaque_pixels = tile.iter().filter(|pixel| **pixel != 0).count();
            if opaque_pixels != 0 {
                stats.drawable_tiles += 1;
                stats.opaque_pixels += opaque_pixels;
            }
        }
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_fills_opaque_black() {
        let mut video = Video::new();
        video.framebuffer[0] = 0xFFFFFFFF;
        video.clear();
        assert!(video.framebuffer.iter().all(|&pixel| pixel == 0xFF000000));
    }

    #[test]
    fn empty_crom_renders_black_frame() {
        let mut video = Video::new();
        let memory = crate::memory::Memory::new();
        video.framebuffer.fill(0xFFFFFFFF);
        video.render_frame(&memory);
        assert!(video.framebuffer.iter().all(|&pixel| pixel == 0xFF000000));
    }

    #[test]
    fn render_frame_uses_active_palette_backdrop() {
        let mut video = Video::new();
        let mut memory = crate::memory::Memory::new();
        let backdrop_offset = 4095 * 2;
        memory.palette_ram[backdrop_offset..backdrop_offset + 2]
            .copy_from_slice(&0x20F0_u16.to_be_bytes());

        video.render_frame(&memory);

        assert!(video.framebuffer.iter().all(|&pixel| pixel == 0xFF00FF00));
    }

    #[test]
    fn crom_without_palette_uses_diagnostic_colors() {
        let mut video = Video::new();
        let mut memory = crate::memory::Memory::new();
        memory.crom = vec![0xFF; BYTES_PER_TILE];

        video.render_frame(&memory);

        assert_eq!(video.framebuffer[0], DIAGNOSTIC_PALETTE[15]);
    }

    #[test]
    fn palette_ram_colors_crom_tiles() {
        let mut video = Video::new();
        let mut memory = crate::memory::Memory::new();
        memory.crom = solid_sprite_tile(3);
        memory.palette_ram[6..8].copy_from_slice(&0x4F00_u16.to_be_bytes());

        video.render_frame(&memory);

        assert_eq!(video.framebuffer[0], 0xFFFF0000);
    }

    #[test]
    fn renderer_uses_active_palette_ram_page() {
        let mut video = Video::new();
        let mut memory = crate::memory::Memory::new();
        memory.crom = solid_sprite_tile(3);
        memory.palette_ram[6..8].copy_from_slice(&0x4F00_u16.to_be_bytes());
        let bank_1_color_3 = crate::memory::PALETTE_RAM_BANK_SIZE + 6;
        memory.palette_ram[bank_1_color_3..bank_1_color_3 + 2]
            .copy_from_slice(&0x20F0_u16.to_be_bytes());
        memory.palette_bank = 1;

        video.render_frame(&memory);

        assert_eq!(video.framebuffer[0], 0xFF00FF00);
    }

    #[test]
    fn renders_simple_sprite_from_vram_control_blocks() {
        let mut video = Video::new();
        let mut memory = crate::memory::Memory::new();
        memory.crom = solid_sprite_tile(3);
        memory.palette_ram[6..8].copy_from_slice(&0x4F00_u16.to_be_bytes());

        write_vram_word_raw(&mut memory, scb1_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb1_attr_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb2_addr(1), 0x0FFF);
        write_vram_word_raw(&mut memory, scb3_addr(1), ((496 - 24) << 7) | 1);
        write_vram_word_raw(&mut memory, scb4_addr(1), 40 << 7);

        video.render_frame(&memory);

        assert_eq!(video.framebuffer[24 * SCREEN_WIDTH + 40], 0xFFFF0000);
        assert_eq!(video.framebuffer[39 * SCREEN_WIDTH + 55], 0xFFFF0000);
        assert_eq!(video.framebuffer[23 * SCREEN_WIDTH + 40], 0xFF000000);
    }

    #[test]
    fn renders_vram_sprites_even_when_palette_bank_is_black() {
        let mut video = Video::new();
        let mut memory = crate::memory::Memory::new();
        memory.crom = solid_sprite_tile(3);
        // Agrandar C-ROM a 512 tiles para evitar mask=0 (que haría que la
        // matriz de diagnóstico llene toda la pantalla con tile 0).
        // Con mask >= 511, los tiles más allá del primer tile son ceros
        // y no afectan el resultado del test.
        memory.crom.resize(512 * BYTES_PER_TILE, 0);
        let backdrop_offset = 4095 * 2;
        memory.palette_ram[backdrop_offset..backdrop_offset + 2]
            .copy_from_slice(&0x20F0_u16.to_be_bytes());

        write_vram_word_raw(&mut memory, scb1_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb1_attr_addr(1, 0), 0x0100);
        write_vram_word_raw(&mut memory, scb2_addr(1), 0x0FFF);
        write_vram_word_raw(&mut memory, scb3_addr(1), ((496 - 24) << 7) | 1);
        write_vram_word_raw(&mut memory, scb4_addr(1), 40 << 7);

        video.framebuffer.fill(0xFF00FF00);
        video.render_sprites_scanline_all(&memory, 24);

        assert_eq!(video.framebuffer[0], 0xFF00FF00);
        assert_eq!(video.framebuffer[24 * SCREEN_WIDTH + 40], 0xFF000000);
    }

    #[test]
    fn sprite_line_buffer_applies_palette_at_present_time() {
        let mut video = Video::new();
        let mut memory = crate::memory::Memory::new();
        memory.crom = solid_sprite_tile(3);
        memory.palette_ram[6..8].copy_from_slice(&0x4F00_u16.to_be_bytes());

        write_vram_word_raw(&mut memory, scb1_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb1_attr_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb2_addr(1), 0x0FFF);
        write_vram_word_raw(&mut memory, scb3_addr(1), ((496 - 24) << 7) | 1);
        write_vram_word_raw(&mut memory, scb4_addr(1), 40 << 7);

        video.reset_sprite_line_buffers();
        video.calculate_buffered_sprites_scanline_all(&memory, 24);
        video.calculate_buffered_sprites_scanline_all(&memory, 25);
        memory.palette_ram[6..8].copy_from_slice(&0x20F0_u16.to_be_bytes());
        video.present_buffered_sprites_scanline(&memory, 24);

        assert_eq!(video.framebuffer[24 * SCREEN_WIDTH + 40], 0xFF00FF00);
    }

    #[test]
    fn render_frame_preserves_final_sprite_line_buffer_phase() {
        let mut video = Video::new();
        let mut memory = crate::memory::Memory::new();
        memory.crom = solid_sprite_tile(3);
        memory.palette_ram[6..8].copy_from_slice(&0x4F00_u16.to_be_bytes());

        write_vram_word_raw(&mut memory, scb1_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb1_attr_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb2_addr(1), 0x0FFF);
        write_vram_word_raw(&mut memory, scb3_addr(1), (273 << 7) | 1);
        write_vram_word_raw(&mut memory, scb4_addr(1), 40 << 7);

        video.render_frame(&memory);

        assert_eq!(
            video.framebuffer[(SCREEN_HEIGHT - 1) * SCREEN_WIDTH + 40],
            0xFFFF0000
        );
        assert_eq!(
            video.framebuffer[(SCREEN_HEIGHT - 2) * SCREEN_WIDTH + 40],
            0xFF000000
        );
    }

    #[test]
    fn applies_scb2_horizontal_and_vertical_shrink() {
        let mut video = Video::new();
        let mut memory = crate::memory::Memory::new();
        memory.crom = solid_sprite_tile(3);
        memory.palette_ram[6..8].copy_from_slice(&0x4F00_u16.to_be_bytes());

        write_vram_word_raw(&mut memory, scb1_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb1_attr_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb2_addr(1), 0x00FF);
        write_vram_word_raw(&mut memory, scb3_addr(1), ((496 - 24) << 7) | 1);
        write_vram_word_raw(&mut memory, scb4_addr(1), 40 << 7);

        video.render_frame(&memory);

        assert_eq!(video.framebuffer[24 * SCREEN_WIDTH + 40], 0xFFFF0000);
        assert_eq!(video.framebuffer[24 * SCREEN_WIDTH + 41], 0xFF000000);

        write_vram_word_raw(&mut memory, scb2_addr(1), 0x0F00);
        video.render_frame(&memory);

        assert_eq!(video.framebuffer[24 * SCREEN_WIDTH + 40], 0xFFFF0000);
        assert_eq!(video.framebuffer[25 * SCREEN_WIDTH + 40], 0xFFFF0000);
        assert_eq!(video.framebuffer[39 * SCREEN_WIDTH + 40], 0xFFFF0000);
        assert_eq!(video.framebuffer[40 * SCREEN_WIDTH + 40], 0xFF000000);
    }

    #[test]
    fn horizontal_shrink_uses_geolith_column_lut() {
        let mut video = Video::new();
        let mut tile = [0u8; SPRITE_TILE_WIDTH * SPRITE_TILE_HEIGHT];
        for (x, pixel) in tile.iter_mut().enumerate().take(SPRITE_TILE_WIDTH) {
            *pixel = (x + 1) as u8;
        }

        let mut palette = [0u32; 16];
        for (index, color) in palette.iter_mut().enumerate() {
            *color = 0xFF000000 | index as u32;
        }

        draw_sprite_tile_scanline(
            &mut video,
            &tile,
            &palette,
            SpriteDrawParams {
                x: 40,
                y: 0,
                h_flip: false,
                v_flip: false,
                transparent_zero: true,
                width: 2,
                height: SPRITE_TILE_HEIGHT,
            },
            0,
            0,
        );

        assert_eq!(video.framebuffer[40], 0xFF000005);
        assert_eq!(video.framebuffer[41], 0xFF000009);
        assert_eq!(video.framebuffer[42], 0x00000000);
    }

    #[test]
    fn horizontal_shrink_lut_matches_geolith() {
        const GEOLITH_HSHRINK_LUT: [[bool; SPRITE_TILE_WIDTH]; 16] = [
            [
                false, false, false, false, false, false, false, false, true, false, false, false,
                false, false, false, false,
            ],
            [
                false, false, false, false, true, false, false, false, true, false, false, false,
                false, false, false, false,
            ],
            [
                false, false, false, false, true, false, false, false, true, false, false, false,
                true, false, false, false,
            ],
            [
                false, false, true, false, true, false, false, false, true, false, false, false,
                true, false, false, false,
            ],
            [
                false, false, true, false, true, false, false, false, true, false, false, false,
                true, false, true, false,
            ],
            [
                false, false, true, false, true, false, true, false, true, false, false, false,
                true, false, true, false,
            ],
            [
                false, false, true, false, true, false, true, false, true, false, true, false,
                true, false, true, false,
            ],
            [
                true, false, true, false, true, false, true, false, true, false, true, false, true,
                false, true, false,
            ],
            [
                true, false, true, false, true, false, true, false, true, true, true, false, true,
                false, true, false,
            ],
            [
                true, false, true, true, true, false, true, false, true, true, true, false, true,
                false, true, false,
            ],
            [
                true, false, true, true, true, false, true, false, true, true, true, false, true,
                false, true, true,
            ],
            [
                true, false, true, true, true, false, true, true, true, true, true, false, true,
                false, true, true,
            ],
            [
                true, false, true, true, true, false, true, true, true, true, true, false, true,
                true, true, true,
            ],
            [
                true, true, true, true, true, false, true, true, true, true, true, false, true,
                true, true, true,
            ],
            [
                true, true, true, true, true, false, true, true, true, true, true, true, true,
                true, true, true,
            ],
            [
                true, true, true, true, true, true, true, true, true, true, true, true, true, true,
                true, true,
            ],
        ];

        assert_eq!(HSHRINK_LUT, GEOLITH_HSHRINK_LUT);
    }

    #[test]
    fn sprite_row_matches_geolith_visible_line_formula() {
        assert_eq!(sprite_row_for_scanline(24, 496 - 24), 0);
        assert_eq!(sprite_row_for_scanline(39, 496 - 24), 15);
        assert_eq!(sprite_row_for_scanline(23, 496 - 24), 511);
    }

    #[test]
    fn l0_zoom_rom_selects_vertical_source_tile_and_row() {
        let mut video = Video::new();
        let mut memory = crate::memory::Memory::new();
        memory.crom = solid_sprite_tile(3);
        memory.crom.extend(solid_sprite_tile(5));
        memory.palette_ram[6..8].copy_from_slice(&0x4F00_u16.to_be_bytes());
        memory.palette_ram[10..12].copy_from_slice(&0x100F_u16.to_be_bytes());
        memory.zoom_rom = vec![0; crate::bios::ZOOM_ROM_SIZE];
        memory.zoom_rom[0x80 * 0x100] = 0x10;

        write_vram_word_raw(&mut memory, scb1_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb1_attr_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb1_addr(1, 1), 1);
        write_vram_word_raw(&mut memory, scb1_attr_addr(1, 1), 0);
        write_vram_word_raw(&mut memory, scb2_addr(1), 0x0F80);
        write_vram_word_raw(&mut memory, scb3_addr(1), ((496 - 24) << 7) | 2);
        write_vram_word_raw(&mut memory, scb4_addr(1), 40 << 7);

        video.render_frame(&memory);

        assert_eq!(video.framebuffer[24 * SCREEN_WIDTH + 40], 0xFF0000FF);
    }

    #[test]
    fn l0_zoom_rom_handles_geolith_size_33_loop_border() {
        let mut zoom_rom = vec![0; crate::bios::ZOOM_ROM_SIZE];
        zoom_rom[0] = 0x12;

        let source_y = sprite_source_y_from_l0(&zoom_rom, 1, 33, 0).unwrap();

        assert_eq!(source_y, 0x12 ^ 0x01FF);
    }

    #[test]
    fn sprite_tile_map_offset_wraps_like_geolith_for_tall_sprites() {
        assert_eq!(sprite_tile_map_offset(0), 0);
        assert_eq!(sprite_tile_map_offset(15), 0);
        assert_eq!(sprite_tile_map_offset(16), 2);
        assert_eq!(sprite_tile_map_offset(511), 62);
        assert_eq!(sprite_tile_map_offset(512), 0);
    }

    #[test]
    fn applies_scb1_auto_animation_to_tile_index() {
        let mut video = Video::new();
        let mut memory = crate::memory::Memory::new();
        memory.crom = solid_sprite_tile(1);
        memory.crom.extend(solid_sprite_tile(2));
        memory.crom.extend(solid_sprite_tile(3));
        memory.crom.extend(solid_sprite_tile(4));
        memory.palette_ram[6..8].copy_from_slice(&0x20F0_u16.to_be_bytes());
        memory.auto_animation_counter.set(2);

        write_vram_word_raw(&mut memory, scb1_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb1_attr_addr(1, 0), 0x0004);
        write_vram_word_raw(&mut memory, scb2_addr(1), 0x0FFF);
        write_vram_word_raw(&mut memory, scb3_addr(1), ((496 - 24) << 7) | 1);
        write_vram_word_raw(&mut memory, scb4_addr(1), 40 << 7);

        video.render_frame(&memory);

        assert_eq!(video.framebuffer[24 * SCREEN_WIDTH + 40], 0xFF00FF00);
    }

    #[test]
    fn renders_sticky_sprite_next_to_driver() {
        let mut video = Video::new();
        let mut memory = crate::memory::Memory::new();
        memory.crom = solid_sprite_tile(3);
        memory.crom.extend(solid_sprite_tile(3));
        memory.palette_ram[6..8].copy_from_slice(&0x4F00_u16.to_be_bytes());

        write_vram_word_raw(&mut memory, scb1_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb1_attr_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb1_addr(2, 0), 1);
        write_vram_word_raw(&mut memory, scb1_attr_addr(2, 0), 0);
        write_vram_word_raw(&mut memory, scb2_addr(1), 0x0FFF);
        write_vram_word_raw(&mut memory, scb2_addr(2), 0x0FFF);
        write_vram_word_raw(&mut memory, scb3_addr(1), ((496 - 24) << 7) | 1);
        write_vram_word_raw(&mut memory, scb3_addr(2), 0x0040);
        write_vram_word_raw(&mut memory, scb4_addr(1), 40 << 7);

        video.render_frame(&memory);

        assert_eq!(video.framebuffer[24 * SCREEN_WIDTH + 55], 0xFFFF0000);
        assert_eq!(video.framebuffer[24 * SCREEN_WIDTH + 56], 0xFFFF0000);
    }

    #[test]
    fn renderer_ignores_sprite_zero() {
        let mut video = Video::new();
        let mut memory = crate::memory::Memory::new();
        memory.crom = solid_sprite_tile(3);
        memory.crom.extend(solid_sprite_tile(3));
        memory.palette_ram[6..8].copy_from_slice(&0x4F00_u16.to_be_bytes());

        write_vram_word_raw(&mut memory, SCB1_START, 0);
        write_vram_word_raw(&mut memory, SCB1_START + 1, 0);
        write_vram_word_raw(&mut memory, SCB2_START, 0x0FFF);
        write_vram_word_raw(&mut memory, SCB3_START, ((496 - 24) << 7) | 1);
        write_vram_word_raw(&mut memory, SCB4_START, 40 << 7);

        write_vram_word_raw(&mut memory, scb1_addr(1, 0), 1);
        write_vram_word_raw(&mut memory, scb1_attr_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb2_addr(1), 0x0FFF);
        write_vram_word_raw(&mut memory, scb3_addr(1), ((496 - 24) << 7) | 1);
        write_vram_word_raw(&mut memory, scb4_addr(1), 80 << 7);
        video.render_frame(&memory);

        assert_eq!(video.framebuffer[24 * SCREEN_WIDTH + 40], 0xFF000000);
        assert_eq!(video.framebuffer[24 * SCREEN_WIDTH + 80], 0xFFFF0000);
    }

    #[test]
    fn scanline_all_sprites_ignores_vram_list_garbage() {
        let mut video = Video::new();
        let mut memory = crate::memory::Memory::new();
        memory.crom = solid_sprite_tile(3);
        memory.crom.resize(512 * BYTES_PER_TILE, 0);
        memory.palette_ram[6..8].copy_from_slice(&0x4F00_u16.to_be_bytes());

        write_vram_word_raw(&mut memory, scb1_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb1_attr_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb2_addr(1), 0x0FFF);
        write_vram_word_raw(&mut memory, scb3_addr(1), ((496 - 24) << 7) | 1);
        write_vram_word_raw(&mut memory, scb4_addr(1), 80 << 7);

        // These VRAM words are normal tile/sprite data for some games.
        // Geolith ignores them as filters and still walks sprite 1..381.
        write_vram_word_raw(&mut memory, 0x8600, 0xFFFF);
        write_vram_word_raw(&mut memory, 0x8680, 0xFFFF);

        video.clear_framebuffer();
        video.render_sprites_scanline_all(&memory, 24);

        assert_eq!(video.framebuffer[24 * SCREEN_WIDTH + 80], 0xFFFF0000);
    }

    #[test]
    fn renders_fix_layer_over_sprites() {
        let mut video = Video::new();
        let mut memory = crate::memory::Memory::new();
        memory.crom = solid_sprite_tile(3);
        memory.srom = blank_fix_tile();
        memory.srom.extend(solid_fix_tile(4));
        memory.palette_ram[6..8].copy_from_slice(&0x4F00_u16.to_be_bytes());
        memory.palette_ram[8..10].copy_from_slice(&0x20F0_u16.to_be_bytes());

        write_vram_word_raw(&mut memory, scb1_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb1_attr_addr(1, 0), 0);
        write_vram_word_raw(&mut memory, scb2_addr(1), 0x0FFF);
        write_vram_word_raw(&mut memory, scb3_addr(1), (496 << 7) | 1);
        write_vram_word_raw(&mut memory, scb4_addr(1), 0);
        write_vram_word_raw(&mut memory, fix_map_address(0, FIX_HIDDEN_TOP_ROWS), 1);

        video.render_frame(&memory);

        assert_eq!(video.framebuffer[0], 0xFF00FF00);
        assert_eq!(video.framebuffer[9], 0xFFFF0000);
    }

    #[test]
    fn lspc_timer_bit_does_not_change_fix_tile_height() {
        let mut normal = Video::new();
        let mut timer_enabled = Video::new();
        let mut memory = crate::memory::Memory::new();
        memory.srom = blank_fix_tile();
        memory.srom.extend(solid_fix_tile(4));
        memory.srom.extend(solid_fix_tile(3));
        memory.palette_ram[6..8].copy_from_slice(&0x4F00_u16.to_be_bytes());
        memory.palette_ram[8..10].copy_from_slice(&0x20F0_u16.to_be_bytes());

        write_vram_word_raw(&mut memory, fix_map_address(0, FIX_HIDDEN_TOP_ROWS), 1);
        write_vram_word_raw(&mut memory, fix_map_address(0, FIX_HIDDEN_TOP_ROWS + 1), 1);

        memory.lspc_mode = 0;
        normal.render_frame(&memory);
        memory.lspc_mode = 0x0020;
        timer_enabled.render_frame(&memory);

        assert_eq!(timer_enabled.framebuffer, normal.framebuffer);
        assert_eq!(timer_enabled.framebuffer[0], 0xFF00FF00);
        assert_eq!(timer_enabled.framebuffer[8 * SCREEN_WIDTH], 0xFF00FF00);
    }

    #[test]
    fn zedblade_fix_filler_entry_is_not_drawn() {
        let mut video = Video::new();
        let mut memory = crate::memory::Memory::new();
        memory.rom_ngh = Some(0x0076);
        memory.srom = vec![0; BYTES_PER_FIX_TILE * 0x21];
        let filler_tile = solid_fix_tile(2);
        memory.srom[0x20 * BYTES_PER_FIX_TILE..0x21 * BYTES_PER_FIX_TILE]
            .copy_from_slice(&filler_tile);

        let palette_f_color_2 = 15 * 32 + 2 * 2;
        memory.palette_ram[palette_f_color_2..palette_f_color_2 + 2]
            .copy_from_slice(&0x7FFF_u16.to_be_bytes());
        write_vram_word_raw(&mut memory, fix_map_address(0, FIX_HIDDEN_TOP_ROWS), 0xF020);

        video.clear_framebuffer();
        video.render_fix_scanline(&memory, 0);
        assert_eq!(video.framebuffer[0], 0xFF000000);

        memory.rom_ngh = None;
        video.clear_framebuffer();
        video.render_fix_scanline(&memory, 0);
        assert_ne!(video.framebuffer[0], 0xFF000000);
    }

    #[test]
    fn fix_layer_can_use_board_sfix_when_latch_selects_it() {
        let mut video = Video::new();
        let mut memory = crate::memory::Memory::new();
        memory.srom = blank_fix_tile();
        memory.srom.extend(solid_fix_tile(3));
        memory.sfix = blank_fix_tile();
        memory.sfix.extend(solid_fix_tile(4));
        memory.palette_ram[6..8].copy_from_slice(&0x4F00_u16.to_be_bytes());
        memory.palette_ram[8..10].copy_from_slice(&0x20F0_u16.to_be_bytes());
        write_vram_word_raw(&mut memory, fix_map_address(0, FIX_HIDDEN_TOP_ROWS), 1);

        memory.use_cart_fix = false;
        video.render_frame(&memory);
        assert_eq!(video.framebuffer[0], 0xFF00FF00);

        memory.use_cart_fix = true;
        video.render_frame(&memory);
        assert_eq!(video.framebuffer[0], 0xFFFF0000);
    }

    #[test]
    fn dynamic_fix_rom_replaces_only_cart_fix_source() {
        let mut memory = crate::memory::Memory::new();
        memory.srom = vec![0x11];
        memory.sfix = vec![0x22];
        memory.dynamic_fix_rom = vec![0x33];

        memory.use_cart_fix = false;
        assert_eq!(active_fix_rom(&memory)[0], 0x22);

        memory.use_cart_fix = true;
        assert_eq!(active_fix_rom(&memory)[0], 0x33);
    }

    #[test]
    fn tile_fix_bankswitch_uses_geolith_visible_row_offset() {
        let mut memory = crate::memory::Memory::new();
        memory.fix_bankswitch = crate::memory::FixBankSwitch::Tile;

        // First visible NGNEON row is hardware map row 2. Geolith's tile
        // banking reads (trow - 1), so row 2 must use bank-control row 1.
        write_vram_word_raw(&mut memory, 0x7500 + 1, 0x0FFF);
        write_vram_word_raw(&mut memory, 0x7500 + 31, 0x0000);

        assert_eq!(fix_tile_bank(&memory, FIX_HIDDEN_TOP_ROWS, 0), 0);
    }

    #[test]
    fn line_fix_bankswitch_uses_geolith_visible_row_offset() {
        let mut memory = crate::memory::Memory::new();
        memory.fix_bankswitch = crate::memory::FixBankSwitch::Line;

        write_vram_word_raw(&mut memory, 0x7500, 0x0200);
        write_vram_word_raw(&mut memory, 0x7580, 0xFFFE);

        assert_eq!(fix_line_bank(&memory, FIX_HIDDEN_TOP_ROWS), 1);
    }

    #[test]
    fn renders_sparse_drawable_fix_tiles_even_with_dense_map_entries() {
        let mut video = Video::new();
        let mut memory = crate::memory::Memory::new();
        memory.srom = blank_fix_tile();
        memory.srom.extend(solid_fix_tile(4));
        memory.srom.extend(blank_fix_tile());
        memory.palette_ram[8..10].copy_from_slice(&0x20F0_u16.to_be_bytes());

        for row in 0..28 {
            // 224px / 8px = 28 rows in 8x8 mode
            let map_row = row + FIX_HIDDEN_TOP_ROWS;
            for col in 0..FIX_MAP_COLUMNS {
                write_vram_word_raw(&mut memory, fix_map_address(col, map_row), 2);
            }
        }
        write_vram_word_raw(&mut memory, fix_map_address(0, FIX_HIDDEN_TOP_ROWS), 1);

        video.render_frame(&memory);

        assert_eq!(video.framebuffer[0], 0xFF00FF00);
        assert_eq!(video.framebuffer[9], 0xFF000000);
    }

    #[test]
    fn calc_crom_mask_produces_wrap_mask() {
        // 0 tiles → mask 0
        assert_eq!(calc_crom_mask(0), 0);
        // 1 tile (128 bytes) → 1 tile, next_power_of_two(1) = 1, mask = 0
        assert_eq!(calc_crom_mask(BYTES_PER_TILE), 0);
        // 2 tiles (256 bytes) → next_power_of_two(2) = 2, mask = 1
        assert_eq!(calc_crom_mask(2 * BYTES_PER_TILE), 1);
        // 640 tiles (81920 bytes) → next_power_of_two(640) = 1024, mask = 1023
        assert_eq!(calc_crom_mask(640 * BYTES_PER_TILE), 1023);
        // 512 tiles → already power of two → mask = 511
        assert_eq!(calc_crom_mask(512 * BYTES_PER_TILE), 511);
        // 16MB (131072 tiles) → mask = 131071 (0x1FFFF)
        assert_eq!(calc_crom_mask(131072 * BYTES_PER_TILE), 131071);
    }

    #[test]
    fn decode_sprite_tile_applies_crom_mask_wrap_around() {
        // Crea un C-ROM con solo 2 tiles (256 bytes)
        let mut crom = vec![0; 2 * BYTES_PER_TILE];
        // Tile 0: pixel sólido color 1 in both stored halves.
        for i in 0..16 {
            crom[i * 4] = 0xFF; // plane0 = all 1s
            crom[64 + i * 4] = 0xFF; // plane0 = all 1s
        }
        // Tile 1: pixel sólido color 1 in both stored halves.
        for i in 0..16 {
            crom[BYTES_PER_TILE + i * 4] = 0xFF; // plane0 del tile 1
            crom[BYTES_PER_TILE + 64 + i * 4] = 0xFF; // plane0 del tile 1
        }

        // tile_index 0 → tile 0 válido
        let tile0 = decode_sprite_tile(&crom, 0, 0).unwrap();
        assert_eq!(tile0[0], 0x01); // plane0 → color 1

        // tile_index 1 → tile 1 válido
        let tile1 = decode_sprite_tile(&crom, 1, 0).unwrap();
        assert_eq!(tile1[0], 0x01); // solo plane0 → color 1

        // tile_index 2 (out of bounds) → máscara 2 & 1 = 0 → tile 0
        let wrapped = decode_sprite_tile(&crom, 2, 0).unwrap();
        assert_eq!(wrapped[0], 0x01);

        // tile_index 3 → máscara 3 & 1 = 1 → tile 1 (solo plane0 → color 1)
        let wrapped3 = decode_sprite_tile(&crom, 3, 0).unwrap();
        assert_eq!(wrapped3[0], 0x01);

        // tile_index 1000000 → máscara 1000000 & 1 = 0 → tile 0
        let wrapped_big = decode_sprite_tile(&crom, 1_000_000, 0).unwrap();
        assert_eq!(wrapped_big[0], 0x01);
    }

    #[test]
    fn decode_sprite_tile_modulos_non_power_of_two_crom_size_like_geolith() {
        let mut crom = vec![0; 3 * BYTES_PER_TILE];
        for y in 0..SPRITE_TILE_HEIGHT {
            // Tile 0: color 1 in both stored halves.
            crom[y * 4] = 0xFF;
            crom[64 + y * 4] = 0xFF;
            // Tile 2: color 2 in both stored halves.
            crom[2 * BYTES_PER_TILE + y * 4 + 2] = 0xFF;
            crom[2 * BYTES_PER_TILE + 64 + y * 4 + 2] = 0xFF;
        }

        // Three tiles produce a power-of-two mask of 3, but Geolith still
        // applies byte offset modulo the real C-ROM size. Index 5 masks to 1,
        // while index 6 masks to 2 and must land on the third tile.
        let masked_to_tile_two = decode_sprite_tile(&crom, 6, 0).unwrap();
        assert_eq!(masked_to_tile_two[0], 0x02);

        let modulo_to_tile_two = decode_sprite_tile(&crom, 5, 0x0000).unwrap();
        assert_eq!(modulo_to_tile_two[0], 0x00);
    }

    #[test]
    fn decode_sprite_tile_ignores_lspc_irqack_and_uses_geolith_data_mask() {
        let mut crom = vec![0; 3 * BYTES_PER_TILE];
        for y in 0..SPRITE_TILE_HEIGHT {
            // Tile 0: color 1.
            crom[y * 4] = 0xFF;
            crom[64 + y * 4] = 0xFF;
            // Tile 1: color 2.
            crom[BYTES_PER_TILE + y * 4 + 2] = 0xFF;
            crom[BYTES_PER_TILE + 64 + y * 4 + 2] = 0xFF;
        }

        // With a register-derived mask, index 4 would wrap to tile 1 in this
        // 3-tile ROM. Geolith uses the data mask: 4 & 3 = 0.
        let tile = decode_sprite_tile(&crom, 4, 1).unwrap();

        assert_eq!(tile[0], 0x01);
    }

    #[test]
    fn decodes_neogeo_sprite_tile_blocks_and_planes() {
        let mut crom = vec![0; BYTES_PER_TILE];

        // NeoGeo C-ROM stores the two 8-pixel halves right-to-left.
        // Decode normalizes display x=0 to the second stored half.
        crom[64] = 0x01; // row 0, displayed left half, plane 0 — LSB set = pixel x=0
        crom[66] = 0x01; // row 0, displayed left half, plane 1 — LSB set = pixel x=0

        let tile = decode_sprite_tile(&crom, 0, 0).unwrap();

        // pixel 0: plane0=1, plane2=0, plane1=1, plane3=0 => 0b0011 = 3
        assert_eq!(tile[0], 0x03);
        // pixel 7: all planes = 0
        assert_eq!(tile[7], 0x00);
        // pixel 8 (right half): all planes = 0
        assert_eq!(tile[8], 0x00);
        assert_eq!(tile[SPRITE_TILE_WIDTH * 8 + 8], 0x00);
    }

    #[test]
    fn decodes_fix_tile_nibbles() {
        let mut srom = blank_fix_tile();
        srom.extend(solid_fix_tile(5));

        let tile = decode_fix_tile(&srom, 1).unwrap();

        assert_eq!(tile[0], 5);
        assert_eq!(tile[FIX_TILE_WIDTH * FIX_TILE_HEIGHT - 1], 5);
    }

    #[test]
    fn debug_vram_stats_counts_core_video_state() {
        let mut memory = crate::memory::Memory::new();
        write_vram_word_raw(&mut memory, scb3_addr(1), ((496 - 24) << 7) | 1);
        write_vram_word_raw(&mut memory, scb2_addr(1), 0x07FF);
        // Unrelated VRAM contents must not filter SCB-derived diagnostics.
        write_vram_word_raw(&mut memory, 0x8600, 2);
        write_vram_word_raw(&mut memory, 0x8680, 0xFFFF);
        write_vram_word_raw(&mut memory, fix_map_address(0, FIX_HIDDEN_TOP_ROWS), 1);
        memory.palette_ram[2] = 0x12;
        memory.palette_ram[3] = 0x34;

        let stats = debug_vram_stats(&memory);

        assert_eq!(stats.sprites_with_height, 1);
        assert_eq!(stats.generated_visible_sprites, 1);
        assert_eq!(stats.generated_sprite_scanlines, 16);
        assert_eq!(stats.generated_max_sprites_per_scanline, 1);
        assert_eq!(stats.generated_overflow_scanlines, 0);
        assert_eq!(stats.sprites_with_horizontal_shrink, 1);
        assert_eq!(stats.sprites_with_vertical_shrink, 0);
        assert_eq!(stats.visible_fix_tiles, 0);
        assert_eq!(stats.drawable_fix_tiles, 0);
        assert_eq!(stats.unique_fix_tiles, 0);
        assert_eq!(stats.fix_opaque_pixels, 0);
        assert_eq!(stats.initialized_palette_banks, 1);
    }

    #[test]
    fn debug_vram_stats_counts_drawable_fix_tiles() {
        let mut memory = crate::memory::Memory::new();
        memory.srom = blank_fix_tile();
        memory.srom.extend(solid_fix_tile(5));
        memory.srom.extend(blank_fix_tile());
        memory.palette_ram[10..12].copy_from_slice(&0x7FFF_u16.to_be_bytes());
        write_vram_word_raw(&mut memory, fix_map_address(0, FIX_HIDDEN_TOP_ROWS), 1);
        write_vram_word_raw(&mut memory, fix_map_address(1, FIX_HIDDEN_TOP_ROWS), 2);

        let stats = debug_vram_stats(&memory);

        assert_eq!(stats.visible_fix_tiles, 2);
        assert_eq!(stats.drawable_fix_tiles, 1);
        assert_eq!(stats.unique_fix_tiles, 2);
        assert_eq!(stats.fix_opaque_pixels, 64);
    }

    #[test]
    fn debug_vram_stats_detects_scanlines_over_lspc_sprite_limit() {
        let mut memory = crate::memory::Memory::new();
        for sprite in 1..=97 {
            write_vram_word_raw(&mut memory, scb3_addr(sprite), ((496 - 24) << 7) | 1);
        }

        let stats = debug_vram_stats(&memory);

        assert_eq!(stats.generated_visible_sprites, 97);
        assert_eq!(stats.generated_sprite_scanlines, 16);
        assert_eq!(stats.generated_max_sprites_per_scanline, 97);
        assert_eq!(stats.generated_overflow_scanlines, 16);
    }

    #[test]
    fn register_to_crom_mask_converts_lspc_romsize_correctly() {
        // rom_size = 0 → 512KB → mask 4095 (0xFFF)
        assert_eq!(register_to_crom_mask(0), 0xFFF);
        // rom_size = 1 → 1MB → mask 8191 (0x1FFF)
        assert_eq!(register_to_crom_mask(1), 0x1FFF);
        // rom_size = 2 → 2MB → mask 16383 (0x3FFF)
        assert_eq!(register_to_crom_mask(2), 0x3FFF);
        // rom_size = 3 → 4MB → mask 32767 (0x7FFF)
        assert_eq!(register_to_crom_mask(3), 0x7FFF);
        // rom_size = 4 → 8MB → mask 65535 (0xFFFF)
        assert_eq!(register_to_crom_mask(4), 0xFFFF);
        // rom_size = 5 → 16MB → mask 131071 (0x1FFFF)
        assert_eq!(register_to_crom_mask(5), 0x1FFFF);
        // rom_size = 7 → 64MB → mask 524287 (0x7FFFF)
        assert_eq!(register_to_crom_mask(7), 0x7FFFF);
    }

    #[test]
    fn verify_crom_mask_register_passes_when_match() {
        let mut memory = crate::memory::Memory::new();
        // 4MB C-ROM → calc_crom_mask = 32767 = register_to_crom_mask(3)
        memory.crom = vec![0; 4 * 1024 * 1024];
        memory.lspc_rom_size = 3;
        assert!(verify_crom_mask_register(&memory));
    }

    #[test]
    fn verify_crom_mask_register_passes_when_register_not_set() {
        let memory = crate::memory::Memory::new();
        // lspc_rom_size == 0 → register not yet written
        assert!(verify_crom_mask_register(&memory));
    }

    #[test]
    fn verify_crom_mask_register_passes_when_register_smaller_than_data() {
        let mut memory = crate::memory::Memory::new();
        // 4MB C-ROM (mask 32767) but register says 1MB (mask 8191)
        // This is fine: the game restricts itself to 1MB window.
        memory.crom = vec![0; 4 * 1024 * 1024];
        memory.lspc_rom_size = 1;
        assert!(verify_crom_mask_register(&memory));
    }

    #[test]
    fn verify_crom_mask_register_warns_when_register_exceeds_data() {
        let mut memory = crate::memory::Memory::new();
        // 1MB C-ROM (mask 8191) but register says 4MB (mask 32767)
        // The game expects more C-ROM than we have → warning.
        memory.crom = vec![0; 1024 * 1024];
        memory.lspc_rom_size = 3;
        assert!(!verify_crom_mask_register(&memory));
    }

    #[test]
    fn decodes_neogeo_palette_word_layout() {
        assert_eq!(decode_palette_color(0x8000), 0xFF000000);
        assert_eq!(decode_palette_color(0x4F00), 0xFFFF0000);
        assert_eq!(decode_palette_color(0x20F0), 0xFF00FF00);
        assert_eq!(decode_palette_color(0x100F), 0xFF0000FF);
        assert_eq!(decode_palette_color(0x7FFF), 0xFFFFFFFF);
        assert_eq!(decode_palette_color(0x0F00), 0xFFF80000);
    }

    #[test]
    fn palette_shadow_latch_attenuates_colors() {
        assert_eq!(decode_palette_color_with_shadow(0x4F00, false), 0xFFFF0000);
        assert_eq!(decode_palette_color_with_shadow(0x4F00, true), 0xFF8E0000);
        assert_eq!(decode_palette_color_with_shadow(0x7FFF, true), 0xFF8E8E8E);
    }

    fn solid_sprite_tile(color: u8) -> Vec<u8> {
        let mut tile = Vec::with_capacity(BYTES_PER_TILE);
        // NeoGeo C-ROM tile format: 128 bytes per 16×16 tile.
        // Left half (x=0..7): bytes 0-63 = 16 rows × 4 bytes/row
        // Right half (x=8..15): bytes 64-127 = 16 rows × 4 bytes/row
        // Per half-row: bytes [plane0, plane2, plane1, plane3]
        for _half in 0..2 {
            for _y in 0..SPRITE_TILE_HEIGHT {
                tile.push(if color & 0x01 != 0 { 0xFF } else { 0x00 }); // plane 0
                tile.push(if color & 0x04 != 0 { 0xFF } else { 0x00 }); // plane 2
                tile.push(if color & 0x02 != 0 { 0xFF } else { 0x00 }); // plane 1
                tile.push(if color & 0x08 != 0 { 0xFF } else { 0x00 }); // plane 3
            }
        }
        tile
    }

    fn blank_fix_tile() -> Vec<u8> {
        vec![0; BYTES_PER_FIX_TILE]
    }

    fn solid_fix_tile(color: u8) -> Vec<u8> {
        let mut tile = vec![0; BYTES_PER_FIX_TILE];
        let packed = (color & 0x0f) | ((color & 0x0f) << 4);
        for y in 0..FIX_TILE_HEIGHT {
            // Interleaved byte layout: [16+y, 24+y, 0+y, 8+y] per row
            tile[16 + y] = packed; // cols 0,1
            tile[24 + y] = packed; // cols 2,3
            tile[y] = packed; // cols 4,5
            tile[8 + y] = packed; // cols 6,7
        }
        tile
    }

    fn scb1_addr(sprite: u16, row: u16) -> u16 {
        SCB1_START + sprite * SCB1_WORDS_PER_SPRITE + row * 2
    }

    fn scb1_attr_addr(sprite: u16, row: u16) -> u16 {
        scb1_addr(sprite, row) + 1
    }

    fn write_vram_word_raw(memory: &mut crate::memory::Memory, address: u16, value: u16) {
        let offset = address as usize * 2;
        let [hi, lo] = value.to_be_bytes();
        memory.vram[offset] = hi;
        memory.vram[offset + 1] = lo;
    }
}

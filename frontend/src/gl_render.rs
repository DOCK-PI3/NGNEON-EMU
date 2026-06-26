//! OpenGL post-processing renderer: barrel-distortion curvature + scanlines +
//! vignette + phosphor bloom in a multi-pass shader pipeline.
//!
//! Pipeline:
//!   1. Upload NeoGeo framebuffer → `fb_tex` (320×224)
//!   2. Bright pass (bloom extraction) → `bloom_bright_tex` (320×224, FBO)
//!   3. Gaussian blur (9-tap at half-res)  → `bloom_blur_tex` (160×112, FBO)
//!   4. Main CRT shader (barrel + scanlines + vignette + bloom composite)

use sdl2::video::Window;
use std::cell::RefCell;
use std::ffi::CString;

const FB_WIDTH: u32 = 320;
const FB_HEIGHT: u32 = 224;
const BLOOM_HALF_W: u32 = 160;
const BLOOM_HALF_H: u32 = 112;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayAspect {
    Original4_3,
    Wide16_9,
}

impl DisplayAspect {
    pub fn from_config(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "16:9" | "wide" | "widescreen" => Self::Wide16_9,
            _ => Self::Original4_3,
        }
    }

    pub fn as_config(self) -> &'static str {
        match self {
            Self::Original4_3 => "4:3",
            Self::Wide16_9 => "16:9",
        }
    }

    pub fn label(self) -> &'static str {
        self.as_config()
    }

    fn ratio(self) -> f32 {
        match self {
            Self::Original4_3 => 4.0 / 3.0,
            Self::Wide16_9 => 16.0 / 9.0,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Original4_3 => Self::Wide16_9,
            Self::Wide16_9 => Self::Original4_3,
        }
    }
}

// ---------------------------------------------------------------------------
// CRT config shared between main loop and the GL renderer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CrtGlConfig {
    pub curvature: bool,
    pub curvature_amount: f32, // 0.0 – 1.0
    pub scanlines: bool,
    pub scanline_intensity: f32, // 0.0 – 1.0
    pub display_aspect: DisplayAspect,
    pub bloom: bool,
    pub bloom_intensity: f32, // 0.0 – 1.0
    pub bloom_threshold: f32, // 0.0 – 1.0 luminance cutoff
}

impl Default for CrtGlConfig {
    fn default() -> Self {
        Self {
            curvature: false,
            curvature_amount: 0.25,
            scanlines: false,
            scanline_intensity: 0.45,
            display_aspect: DisplayAspect::Original4_3,
            bloom: false,
            bloom_intensity: 0.40,
            bloom_threshold: 0.60,
        }
    }
}

// ---------------------------------------------------------------------------
// Shader sources
// ---------------------------------------------------------------------------

/// Shared vertex shader — all passes render the same fullscreen quad.
const VERTEX_SHADER: &str = r#"#version 330 core

layout(location = 0) in vec2 aPos;
layout(location = 1) in vec2 aTexCoord;

out vec2 TexCoord;

void main() {
    gl_Position = vec4(aPos, 0.0, 1.0);
    TexCoord = aTexCoord;
}
"#;

/// Bright-pass fragment shader: extracts pixels above a luminance threshold.
const BRIGHT_PASS_SHADER: &str = r#"#version 330 core

in vec2 TexCoord;
out vec4 FragColor;

uniform sampler2D uTex;
uniform float uThreshold;

void main() {
    vec4 color = texture(uTex, TexCoord);
    // ITU-R BT.709 luminance weights
    float lum = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));
    if (lum > uThreshold) {
        FragColor = color;
    } else {
        FragColor = vec4(0.0, 0.0, 0.0, 0.0);
    }
}
"#;

/// 9-tap Gaussian blur fragment shader (single-pass, applied at half-res).
const BLUR_SHADER: &str = r#"#version 330 core

in vec2 TexCoord;
out vec4 FragColor;

uniform sampler2D uTex;
uniform vec2 uTexelSize; // 1.0 / texture_dimensions

void main() {
    // 9-tap Gaussian kernel (σ ≈ 1.5, normalized)
    vec2 offsets[9] = vec2[](
        vec2(-1.5, -1.5), vec2( 0.0, -1.5), vec2( 1.5, -1.5),
        vec2(-1.5,  0.0), vec2( 0.0,  0.0), vec2( 1.5,  0.0),
        vec2(-1.5,  1.5), vec2( 0.0,  1.5), vec2( 1.5,  1.5)
    );
    float weights[9] = float[](
        0.0625, 0.125, 0.0625,
        0.125,  0.25,  0.125,
        0.0625, 0.125, 0.0625
    );

    vec4 sum = vec4(0.0);
    for (int i = 0; i < 9; i++) {
        sum += texture(uTex, TexCoord + offsets[i] * uTexelSize) * weights[i];
    }
    FragColor = sum;
}
"#;

/// Main CRT fragment shader: barrel-distortion + scanlines + vignette + bloom composite.
///
/// Uniforms:
///   uTex             : sampler2D — the NeoGeo framebuffer (320×224)
///   uBloomTex        : sampler2D — bloom blur texture (160×112)
///   uCurvatureAmt    : float     — 0.0 = off
///   uScanlineAmt     : float     — 0.0 = off
///   uBloomIntensity  : float     — 0.0 = off
///   uOutputSize      : vec2      — output resolution in pixels
const CRT_FRAGMENT_SHADER: &str = r#"#version 330 core

in vec2 TexCoord;
out vec4 FragColor;

uniform sampler2D uTex;
uniform sampler2D uBloomTex;
uniform float uCurvatureAmt;
uniform float uScanlineAmt;
uniform float uBloomIntensity;
uniform vec2 uOutputSize;

void main() {
    // --- barrel distortion ---
    vec2 uv = TexCoord;
    if (uCurvatureAmt > 0.001) {
        vec2 centered = uv - 0.5;
        float r2 = dot(centered, centered);
        float k = uCurvatureAmt * 0.35;
        float factor = 1.0 + k * r2;
        uv = centered * factor + 0.5;

        // Clamp to avoid wrapping artefacts
        if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
            FragColor = vec4(0.0, 0.0, 0.0, 1.0);
            return;
        }
    }

    vec4 color = texture(uTex, uv);

    // --- bloom composite ---
    if (uBloomIntensity > 0.001) {
        // Bloom texture is at half-res; sampling with the same UV works
        // because GL automatically interpolates the lower-resolution texture
        vec4 bloom = texture(uBloomTex, uv);
        color.rgb += bloom.rgb * uBloomIntensity;
    }

    // --- scanlines ---
    if (uScanlineAmt > 0.001) {
        // Determine which scanline (row) we are on in output space
        float row = TexCoord.y * uOutputSize.y;
        float phase = fract(row * 0.5);
        // Darken every other line
        float scanFactor = mix(1.0, 0.45, uScanlineAmt * step(0.5, phase));
        color.rgb *= scanFactor;
    }

    // --- vignette (only when curvature is active) ---
    if (uCurvatureAmt > 0.001) {
        vec2 centered = TexCoord - 0.5;
        float r = length(centered / 0.707); // 0.707 = sqrt(0.5), corner distance
        float vignette = 1.0 - r * 0.50 * uCurvatureAmt;
        color.rgb *= vignette;
    }

    FragColor = color;
}
"#;

// ---------------------------------------------------------------------------
// OpenGL renderer
// ---------------------------------------------------------------------------

struct CrtUniforms {
    curvature_amt: i32,
    scanline_amt: i32,
    output_size: i32,
    bloom_intensity: i32,
}

pub struct CrtGlRenderer {
    _gl_context: sdl2::video::GLContext,
    /// Fullscreen quad
    vao: u32,
    _vbo: u32,
    /// NeoGeo framebuffer texture (320×224, ARGB data swizzled to RGBA on CPU)
    fb_tex: u32,
    /// Main CRT shader program
    crt_program: u32,
    crt_uniforms: CrtUniforms,
    /// Bloom pipeline
    bloom_bright_fbo: u32,
    bloom_bright_tex: u32,
    bloom_blur_fbo: u32,
    bloom_blur_tex: u32,
    bright_program: u32,
    bright_uni_threshold: i32,
    blur_program: u32,
    blur_uni_texel_size: i32,
    /// Pre-allocated buffer for ARGB→RGBA swizzle (reused every frame)
    swizzle_buf: RefCell<Vec<u32>>,
}

impl CrtGlRenderer {
    /// Initialize OpenGL, compile shaders, and create the rendering resources.
    ///
    /// **Important:** GL attributes (version, profile, etc.) must be set on the
    /// video subsystem **before** the window is created — the caller (`main.rs`)
    /// is responsible for that. This function only creates the context from the
    /// already-created window.
    ///
    /// `video` is the SDL2 video subsystem (used for gl_get_proc_address).
    /// `window` must be an OpenGL-capable window (created with `.opengl()`).
    pub fn new(video: &sdl2::VideoSubsystem, window: &Window) -> Result<Self, String> {
        let gl_context = window
            .gl_create_context()
            .map_err(|e| format!("GL context creation failed: {e}"))?;

        gl::load_with(|name| video.gl_get_proc_address(name) as *const _);
        // The frontend owns pacing at the exact Neo Geo refresh rate. Leaving
        // driver VSync enabled can add a second wait at the monitor refresh
        // rate and make video and audio run slightly slow on 60 Hz displays.
        if let Err(error) = video.gl_set_swap_interval(sdl2::video::SwapInterval::Immediate) {
            eprintln!(
                "[WARN] El driver no permitió desactivar VSync; el pacing manual seguirá activo: {error}"
            );
        }

        // -- shader programs --------------------------------------------------

        let vert = compile_shader(gl::VERTEX_SHADER, VERTEX_SHADER)?;

        let crt_frag = compile_shader(gl::FRAGMENT_SHADER, CRT_FRAGMENT_SHADER)?;
        let crt_program = link_program(vert, crt_frag)?;

        let bright_frag = compile_shader(gl::FRAGMENT_SHADER, BRIGHT_PASS_SHADER)?;
        let bright_program = link_program_keep_vert(vert, bright_frag)?;

        let blur_frag = compile_shader(gl::FRAGMENT_SHADER, BLUR_SHADER)?;
        let blur_program = link_program_keep_vert(vert, blur_frag)?;

        // -- uniforms --------------------------------------------------------

        let crt_uniforms = CrtUniforms {
            curvature_amt: get_uniform(crt_program, "uCurvatureAmt")?,
            scanline_amt: get_uniform(crt_program, "uScanlineAmt")?,
            output_size: get_uniform(crt_program, "uOutputSize")?,
            bloom_intensity: get_uniform(crt_program, "uBloomIntensity")?,
        };
        let bright_uni_threshold = get_uniform(bright_program, "uThreshold")?;
        let blur_uni_texel_size = get_uniform(blur_program, "uTexelSize")?;

        // -- VAO (fullscreen quad) -------------------------------------------

        let (vao, vbo) = create_fullscreen_quad();

        // -- textures --------------------------------------------------------

        let fb_tex = create_rgba_texture(FB_WIDTH, FB_HEIGHT, gl::NEAREST);
        let bloom_bright_tex = create_rgba_texture(FB_WIDTH, FB_HEIGHT, gl::LINEAR);
        let bloom_blur_tex = create_rgba_texture(BLOOM_HALF_W, BLOOM_HALF_H, gl::LINEAR);

        // -- FBOs ------------------------------------------------------------

        let bloom_bright_fbo = create_fbo(bloom_bright_tex);
        let bloom_blur_fbo = create_fbo(bloom_blur_tex);

        // -- bind sampler uniforms (static, set once) ------------------------

        unsafe {
            // CRT shader: uTex → unit 0, uBloomTex → unit 1
            gl::UseProgram(crt_program);
            let name_tex = CString::new("uTex").unwrap();
            let loc = gl::GetUniformLocation(crt_program, name_tex.as_ptr());
            if loc != -1 {
                gl::Uniform1i(loc, 0);
            }
            let name_bloom = CString::new("uBloomTex").unwrap();
            let loc2 = gl::GetUniformLocation(crt_program, name_bloom.as_ptr());
            if loc2 != -1 {
                gl::Uniform1i(loc2, 1);
            }
            gl::UseProgram(0);

            // Bright-pass shader: uTex → unit 0
            gl::UseProgram(bright_program);
            let name_tex = CString::new("uTex").unwrap();
            let loc = gl::GetUniformLocation(bright_program, name_tex.as_ptr());
            if loc != -1 {
                gl::Uniform1i(loc, 0);
            }
            gl::UseProgram(0);

            // Blur shader: uTex → unit 0
            gl::UseProgram(blur_program);
            let name_tex = CString::new("uTex").unwrap();
            let loc = gl::GetUniformLocation(blur_program, name_tex.as_ptr());
            if loc != -1 {
                gl::Uniform1i(loc, 0);
            }
            gl::UseProgram(0);
        }

        Ok(Self {
            _gl_context: gl_context,
            vao,
            _vbo: vbo,
            fb_tex,
            crt_program,
            crt_uniforms,
            bloom_bright_fbo,
            bloom_bright_tex,
            bloom_blur_fbo,
            bloom_blur_tex,
            bright_program,
            bright_uni_threshold,
            blur_program,
            blur_uni_texel_size,
            swizzle_buf: RefCell::new(Vec::with_capacity(320 * 224)),
        })
    }

    /// Upload the NeoGeo framebuffer and render with the current CRT settings.
    ///
    /// `framebuffer` must be 320×224 ARGB8888 pixels.
    pub fn render(
        &self,
        framebuffer: &[u32],
        fb_width: u32,
        fb_height: u32,
        config: &CrtGlConfig,
        output_width: u32,
        output_height: u32,
    ) {
        assert_eq!(framebuffer.len(), (FB_WIDTH * FB_HEIGHT) as usize);
        assert_eq!(fb_width, FB_WIDTH);
        assert_eq!(fb_height, FB_HEIGHT);

        unsafe {
            let bloom_enabled = config.bloom && config.bloom_intensity > 0.0;

            // ----------------------------------------------------------------
            // Step 1: swizzle NeoGeo framebuffer (0xAARRGGBB) → RGBA byte
            //         order, then upload to fb_tex (texture unit 0).
            //
            // 0xAARRGGBB in little-endian memory → [BB,GG,RR,AA].
            // gl::BGRA uploads are unreliable on some Windows drivers, so
            // we manually rebuild each u32 as 0xAABBGGRR → [RR,GG,BB,AA]
            // and upload with universally-supported gl::RGBA.
            // ----------------------------------------------------------------
            {
                let mut buf = self.swizzle_buf.borrow_mut();
                buf.clear();
                buf.extend(framebuffer.iter().map(|&p| {
                    let a = (p >> 24) & 0xFF;
                    let r = (p >> 16) & 0xFF;
                    let g = (p >> 8) & 0xFF;
                    let b = p & 0xFF;
                    (a << 24) | (b << 16) | (g << 8) | r
                }));
                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, self.fb_tex);
                gl::TexSubImage2D(
                    gl::TEXTURE_2D,
                    0,
                    0,
                    0,
                    fb_width as i32,
                    fb_height as i32,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    buf.as_ptr() as *const _,
                );
            }

            if bloom_enabled {
                // ------------------------------------------------------------
                // Step 2a: bright-pass → bloom_bright_tex (320×224)
                // ------------------------------------------------------------
                gl::BindFramebuffer(gl::FRAMEBUFFER, self.bloom_bright_fbo);
                gl::Viewport(0, 0, FB_WIDTH as i32, FB_HEIGHT as i32);

                gl::UseProgram(self.bright_program);
                gl::Uniform1f(self.bright_uni_threshold, config.bloom_threshold);
                gl::BindVertexArray(self.vao);
                gl::DrawArrays(gl::TRIANGLES, 0, 6);

                // ------------------------------------------------------------
                // Step 2b: Gaussian blur → bloom_blur_tex (160×112 half-res)
                //          Reading from bloom_bright_tex (unit 0)
                // ------------------------------------------------------------
                gl::BindFramebuffer(gl::FRAMEBUFFER, self.bloom_blur_fbo);
                gl::Viewport(0, 0, BLOOM_HALF_W as i32, BLOOM_HALF_H as i32);

                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, self.bloom_bright_tex);

                gl::UseProgram(self.blur_program);
                gl::Uniform2f(
                    self.blur_uni_texel_size,
                    1.0 / FB_WIDTH as f32,
                    1.0 / FB_HEIGHT as f32,
                );
                gl::BindVertexArray(self.vao);
                gl::DrawArrays(gl::TRIANGLES, 0, 6);

                // Bind bloom_blur_tex to texture unit 1 for the CRT shader
                gl::ActiveTexture(gl::TEXTURE1);
                gl::BindTexture(gl::TEXTURE_2D, self.bloom_blur_tex);
                // Switch back to unit 0 for fb_tex
                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, self.fb_tex);
            }

            // ----------------------------------------------------------------
            // Step 3: main CRT shader → screen (no FBO, viewport to output)
            // ----------------------------------------------------------------
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::Viewport(0, 0, output_width as i32, output_height as i32);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            let viewport = aspect_viewport(output_width, output_height, config.display_aspect);
            gl::Viewport(viewport.x, viewport.y, viewport.width, viewport.height);

            gl::UseProgram(self.crt_program);

            let curvature = if config.curvature {
                config.curvature_amount
            } else {
                0.0
            };
            let scanlines = if config.scanlines {
                config.scanline_intensity
            } else {
                0.0
            };
            let bloom = if bloom_enabled {
                config.bloom_intensity
            } else {
                0.0
            };

            gl::Uniform1f(self.crt_uniforms.curvature_amt, curvature);
            gl::Uniform1f(self.crt_uniforms.scanline_amt, scanlines);
            gl::Uniform1f(self.crt_uniforms.bloom_intensity, bloom);
            gl::Uniform2f(
                self.crt_uniforms.output_size,
                viewport.width as f32,
                viewport.height as f32,
            );

            gl::BindVertexArray(self.vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);

            // Clean up GL state
            gl::BindVertexArray(0);
            gl::UseProgram(0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::ActiveTexture(gl::TEXTURE0);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AspectViewport {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub fn aspect_viewport(
    output_width: u32,
    output_height: u32,
    aspect: DisplayAspect,
) -> AspectViewport {
    if output_width == 0 || output_height == 0 {
        return AspectViewport {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };
    }

    let target = aspect.ratio();
    let output = output_width as f32 / output_height as f32;

    let (width, height) = if output > target {
        (
            (output_height as f32 * target).round() as u32,
            output_height,
        )
    } else {
        (output_width, (output_width as f32 / target).round() as u32)
    };

    AspectViewport {
        x: ((output_width - width) / 2) as i32,
        y: ((output_height - height) / 2) as i32,
        width: width as i32,
        height: height as i32,
    }
}

impl Drop for CrtGlRenderer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.crt_program);
            gl::DeleteProgram(self.bright_program);
            gl::DeleteProgram(self.blur_program);
            gl::DeleteFramebuffers(1, &self.bloom_bright_fbo);
            gl::DeleteFramebuffers(1, &self.bloom_blur_fbo);
            gl::DeleteTextures(1, &self.fb_tex);
            gl::DeleteTextures(1, &self.bloom_bright_tex);
            gl::DeleteTextures(1, &self.bloom_blur_tex);
            gl::DeleteVertexArrays(1, &self.vao);
            gl::DeleteBuffers(1, &self._vbo);
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn compile_shader(shader_type: u32, source: &str) -> Result<u32, String> {
    unsafe {
        let shader = gl::CreateShader(shader_type);
        let c_source = CString::new(source).unwrap();
        gl::ShaderSource(shader, 1, &c_source.as_ptr(), std::ptr::null());
        gl::CompileShader(shader);

        let mut success: i32 = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
        if success == 0 {
            let mut log_len: i32 = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut log_len);
            let mut log = vec![0u8; log_len as usize];
            gl::GetShaderInfoLog(
                shader,
                log_len,
                std::ptr::null_mut(),
                log.as_mut_ptr() as *mut _,
            );
            let msg = String::from_utf8_lossy(&log);
            gl::DeleteShader(shader);
            let kind = if shader_type == gl::VERTEX_SHADER {
                "vertex"
            } else {
                "fragment"
            };
            return Err(format!("{kind} shader compilation failed:\n{msg}"));
        }
        Ok(shader)
    }
}

/// Link a vertex + fragment shader into a program. The vertex shader is reused
/// across programs, so it is NOT deleted after linking.
fn link_program(vert: u32, frag: u32) -> Result<u32, String> {
    let program = link_program_impl(vert, frag)?;
    unsafe {
        gl::DeleteShader(frag);
    }
    Ok(program)
}

/// Like link_program but the vertex shader is kept alive (for reuse).
fn link_program_keep_vert(_vert: u32, frag: u32) -> Result<u32, String> {
    let program = link_program_impl(_vert, frag)?;
    unsafe {
        gl::DeleteShader(frag);
    }
    Ok(program)
}

fn link_program_impl(vert: u32, frag: u32) -> Result<u32, String> {
    unsafe {
        let program = gl::CreateProgram();
        gl::AttachShader(program, vert);
        gl::AttachShader(program, frag);
        gl::LinkProgram(program);

        let mut success: i32 = 0;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
        if success == 0 {
            let mut log_len: i32 = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut log_len);
            let mut log = vec![0u8; log_len as usize];
            gl::GetProgramInfoLog(
                program,
                log_len,
                std::ptr::null_mut(),
                log.as_mut_ptr() as *mut _,
            );
            let msg = String::from_utf8_lossy(&log);
            gl::DeleteProgram(program);
            return Err(format!("shader linking failed:\n{msg}"));
        }
        Ok(program)
    }
}

fn get_uniform(program: u32, name: &str) -> Result<i32, String> {
    let c_name = CString::new(name).unwrap();
    let loc = unsafe { gl::GetUniformLocation(program, c_name.as_ptr() as *const _) };
    if loc == -1 {
        return Err(format!("uniform '{name}' not found (may be optimized out)"));
    }
    Ok(loc)
}

/// Fullscreen quad: two triangles covering NDC (-1..1).
fn create_fullscreen_quad() -> (u32, u32) {
    #[rustfmt::skip]
    let vertices: [f32; 24] = [
        // pos         // tex coord
        -1.0, -1.0,    0.0, 1.0,
         1.0, -1.0,    1.0, 1.0,
        -1.0,  1.0,    0.0, 0.0,

        -1.0,  1.0,    0.0, 0.0,
         1.0, -1.0,    1.0, 1.0,
         1.0,  1.0,    1.0, 0.0,
    ];

    unsafe {
        let mut vao: u32 = 0;
        let mut vbo: u32 = 0;
        gl::GenVertexArrays(1, &mut vao);
        gl::GenBuffers(1, &mut vbo);

        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * std::mem::size_of::<f32>()) as isize,
            vertices.as_ptr() as *const _,
            gl::STATIC_DRAW,
        );
        gl::VertexAttribPointer(
            0,
            2,
            gl::FLOAT,
            gl::FALSE,
            (4 * std::mem::size_of::<f32>()) as i32,
            std::ptr::null(),
        );
        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(
            1,
            2,
            gl::FLOAT,
            gl::FALSE,
            (4 * std::mem::size_of::<f32>()) as i32,
            (2 * std::mem::size_of::<f32>()) as *const _,
        );
        gl::EnableVertexAttribArray(1);

        gl::BindVertexArray(0);
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        (vao, vbo)
    }
}

/// Create an RGBA8 2D texture with the requested filtering mode.
fn create_rgba_texture(w: u32, h: u32, filter: u32) -> u32 {
    unsafe {
        let mut tex: u32 = 0;
        gl::GenTextures(1, &mut tex);
        gl::BindTexture(gl::TEXTURE_2D, tex);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA8 as i32,
            w as i32,
            h as i32,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            std::ptr::null(),
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, filter as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, filter as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::BindTexture(gl::TEXTURE_2D, 0);
        tex
    }
}

/// Create an FBO with the given texture as its colour attachment.
fn create_fbo(color_tex: u32) -> u32 {
    unsafe {
        let mut fbo: u32 = 0;
        gl::GenFramebuffers(1, &mut fbo);
        gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D,
            color_tex,
            0,
        );
        let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
        if status != gl::FRAMEBUFFER_COMPLETE {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::DeleteFramebuffers(1, &fbo);
            panic!("FBO incomplete: status 0x{status:X}");
        }
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        fbo
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crt_gl_config_defaults() {
        let cfg = CrtGlConfig::default();
        assert!(!cfg.curvature);
        assert!(!cfg.scanlines);
        assert_eq!(cfg.display_aspect, DisplayAspect::Original4_3);
        assert!(!cfg.bloom);
        assert!((cfg.curvature_amount - 0.25).abs() < 0.001);
        assert!((cfg.scanline_intensity - 0.45).abs() < 0.001);
        assert!((cfg.bloom_intensity - 0.40).abs() < 0.001);
        assert!((cfg.bloom_threshold - 0.60).abs() < 0.001);
    }

    #[test]
    fn crt_gl_config_clone() {
        let cfg = CrtGlConfig {
            curvature: true,
            curvature_amount: 0.5,
            scanlines: true,
            scanline_intensity: 0.8,
            display_aspect: DisplayAspect::Wide16_9,
            bloom: true,
            bloom_intensity: 0.6,
            bloom_threshold: 0.4,
        };
        let cfg2 = cfg.clone();
        assert_eq!(cfg2.bloom, cfg.bloom);
        assert!((cfg2.bloom_intensity - cfg.bloom_intensity).abs() < 0.001);
    }

    #[test]
    fn bloom_half_res_dimensions() {
        assert_eq!(FB_WIDTH / 2, BLOOM_HALF_W);
        assert_eq!(FB_HEIGHT / 2, BLOOM_HALF_H);
    }

    #[test]
    fn aspect_viewport_pillarboxes_to_4_3() {
        let vp = aspect_viewport(1920, 1080, DisplayAspect::Original4_3);
        assert_eq!(
            vp,
            AspectViewport {
                x: 240,
                y: 0,
                width: 1440,
                height: 1080,
            }
        );
    }

    #[test]
    fn aspect_viewport_uses_full_16_9_output() {
        let vp = aspect_viewport(1920, 1080, DisplayAspect::Wide16_9);
        assert_eq!(
            vp,
            AspectViewport {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            }
        );
    }

    #[test]
    fn display_aspect_config_roundtrip() {
        assert_eq!(DisplayAspect::from_config("16:9"), DisplayAspect::Wide16_9);
        assert_eq!(
            DisplayAspect::from_config("4:3"),
            DisplayAspect::Original4_3
        );
        assert_eq!(DisplayAspect::Wide16_9.as_config(), "16:9");
    }
}

import type { SplatDisplacement } from '../types.ts';

export type SplatVertex = {
  position: [number, number, number];
  uv: [number, number];
  normal: [number, number, number];
  displacementOffset: [number, number, number];
};

export type SplatUniforms = {
  modelViewMatrix: Float32Array | ArrayLike<number>;
  projectionMatrix: Float32Array | ArrayLike<number>;
  displacementScale: number;
  displacementLimit: number;
};

const VERTEX_STRIDE = 11 * 4;
const MAX_SPLATS = 256;
const VERTS_PER_SPLAT = 4;
const INDICES_PER_SPLAT = 6;

export class SplatRenderer {
  private device: GPUDevice;
  private vertexBuffer: GPUBuffer;
  private indexBuffer: GPUBuffer;
  private uniformBuffer: GPUBuffer;
  private pipeline: GPURenderPipeline;
  private bindGroup: GPUBindGroup;

  private splatCount = 0;
  private displacements = new Map<number, SplatDisplacement>();

  private vertexData: Float32Array;

  constructor(device: GPUDevice) {
    this.device = device;

    const totalVerts = MAX_SPLATS * VERTS_PER_SPLAT;
    this.vertexData = new Float32Array(totalVerts * 11);

    this.vertexBuffer = device.createBuffer({
      size: totalVerts * VERTEX_STRIDE,
      usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
    });

    const indexData = new Uint16Array(MAX_SPLATS * INDICES_PER_SPLAT);
    for (let i = 0; i < MAX_SPLATS; i++) {
      const base = i * 4;
      const ibase = i * 6;
      indexData[ibase] = base;
      indexData[ibase + 1] = base + 1;
      indexData[ibase + 2] = base + 2;
      indexData[ibase + 3] = base + 2;
      indexData[ibase + 4] = base + 3;
      indexData[ibase + 5] = base;
    }
    this.indexBuffer = device.createBuffer({
      size: indexData.byteLength,
      usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST,
    });
    device.queue.writeBuffer(this.indexBuffer, 0, indexData);

    this.uniformBuffer = device.createBuffer({
      size: 144,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });

    this.pipeline = this.createRenderPipeline();
    this.bindGroup = device.createBindGroup({
      layout: this.pipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: this.uniformBuffer } },
      ],
    });
  }

  private createRenderPipeline(): GPURenderPipeline {
    const vertexShader = this.device.createShaderModule({
      code: `
        struct Uniforms {
          model_view: mat4x4<f32>,
          projection: mat4x4<f32>,
          displacement_scale: f32,
          displacement_limit: f32,
        }

        @group(0) @binding(0) var<uniform> uniforms: Uniforms;

        struct VertexInput {
          @location(0) position: vec3<f32>,
          @location(1) uv: vec2<f32>,
          @location(2) normal: vec3<f32>,
          @location(3) displacement_offset: vec3<f32>,
        }

        struct VertexOutput {
          @builtin(position) clip_position: vec4<f32>,
          @location(0) uv: vec2<f32>,
          @location(1) normal: vec3<f32>,
        }

        @vertex
        fn main(input: VertexInput) -> VertexOutput {
          let displacement: f32 = input.displacement_offset.z;
          let clamped: f32 = min(displacement, uniforms.displacement_limit);
          let offset: vec3<f32> = input.normal * clamped * uniforms.displacement_scale;

          let world_pos: vec4<f32> = vec4<f32>(input.position + offset, 1.0);
          var output: VertexOutput;
          output.clip_position = uniforms.projection * uniforms.model_view * world_pos;
          output.uv = input.uv;
          output.normal = input.normal;
          return output;
        }
      `,
    });

    const fragmentShader = this.device.createShaderModule({
      code: `
        @group(0) @binding(0) var<uniform> uniforms: Uniforms;

        struct FragmentInput {
          @location(0) uv: vec2<f32>,
          @location(1) normal: vec3<f32>,
        }

        @fragment
        fn main(input: FragmentInput) -> @location(0) vec4<f32> {
          let base_color: vec4<f32> = vec4<f32>(0.85, 0.85, 0.92, 1.0);
          let ambient: f32 = 0.4;
          let ndotl: f32 = max(dot(normalize(input.normal), vec3<f32>(0.0, 0.0, 1.0)), 0.0);
          let lighting: f32 = ambient + (1.0 - ambient) * ndotl;
          return vec4<f32>(base_color.rgb * lighting, base_color.a);
        }
      `,
    });

    return this.device.createRenderPipeline({
      layout: 'auto',
      vertex: {
        module: vertexShader,
        entryPoint: 'main',
        buffers: [
          {
            arrayStride: VERTEX_STRIDE,
            attributes: [
              { shaderLocation: 0, offset: 0, format: 'float32x3' },
              { shaderLocation: 1, offset: 12, format: 'float32x2' },
              { shaderLocation: 2, offset: 20, format: 'float32x3' },
              { shaderLocation: 3, offset: 32, format: 'float32x3' },
            ],
          },
        ],
      },
      fragment: {
        module: fragmentShader,
        entryPoint: 'main',
        targets: [{ format: navigator.gpu?.getPreferredCanvasFormat() ?? 'bgra8unorm' }],
      },
      primitive: { topology: 'triangle-list' },
    });
  }

  applyDisplacement(displacements: Map<number, SplatDisplacement>): void {
    this.displacements = new Map(displacements);

    const numActive = this.displacements.size;
    if (numActive === 0) return;

    let vertIdx = 0;
    for (const [pointerId, disp] of this.displacements) {
      if (vertIdx >= MAX_SPLATS) break;

      const base = vertIdx * 11;
      const halfSize = 8;

      const ox = disp.offsetX;
      const oy = disp.offsetY;
      const d = disp.displacement;

      const verts: Array<[number, number, number, number, number, number, number, number, number, number, number]> = [
        [ox - halfSize, oy - halfSize, 0, 0, 0, 0, 0, -1, 0, 0, d],
        [ox + halfSize, oy - halfSize, 0, 1, 0, 0, 0, -1, 0, 0, d],
        [ox + halfSize, oy + halfSize, 0, 1, 1, 0, 0, -1, 0, 0, d],
        [ox - halfSize, oy + halfSize, 0, 0, 1, 0, 0, -1, 0, 0, d],
      ];

      for (let v = 0; v < 4; v++) {
        const vbase = base + v * 11;
        for (let c = 0; c < 11; c++) {
          this.vertexData[vbase + c] = verts[v][c];
        }
      }

      vertIdx++;
    }

    this.splatCount = vertIdx;
    const uploadSize = vertIdx * VERTS_PER_SPLAT * VERTEX_STRIDE;
    this.device.queue.writeBuffer(this.vertexBuffer, 0, this.vertexData, 0, uploadSize / 4);
  }

  updateUniforms(uniforms: SplatUniforms): void {
    const data = new Float32Array(36);
    data.set(uniforms.modelViewMatrix, 0);
    data.set(uniforms.projectionMatrix, 16);
    data[32] = uniforms.displacementScale;
    data[33] = uniforms.displacementLimit;
    this.device.queue.writeBuffer(this.uniformBuffer, 0, data);
  }

  render(pass: GPURenderPassEncoder): void {
    if (this.splatCount === 0) return;

    pass.setPipeline(this.pipeline);
    pass.setBindGroup(0, this.bindGroup);
    pass.setVertexBuffer(0, this.vertexBuffer);
    pass.setIndexBuffer(this.indexBuffer, 'uint16');
    pass.drawIndexed(this.splatCount * INDICES_PER_SPLAT, 1, 0, 0, 0);
  }

  destroy(): void {
    this.vertexBuffer.destroy();
    this.indexBuffer.destroy();
    this.uniformBuffer.destroy();
    this.displacements.clear();
  }
}

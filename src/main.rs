use std::time::Instant;

async fn run()
{
    // Get file name from input args
    let args: Vec<String> = std::env::args().collect();
    let file: &str = &args[1];

    // Convert image into a vector of bytes and get its dimensions
    let image_file = image::open(std::path::Path::new(file)).unwrap().to_rgba();
    let (width, height) = image_file.dimensions();
    let len = (width * height * 4) as u64;
    let in_buf = image_file.into_raw();

    // Connect to graphics adapter
    let adapter = wgpu::Adapter::request(
        &wgpu::RequestAdapterOptions
        {
            power_preference: wgpu::PowerPreference::HighPerformance,
            backends: wgpu::BackendBit::PRIMARY
        }
    ).unwrap();
    println!("{:?}", adapter.get_info());

    // Get device and queue pointers
    let (device, mut queue) = adapter.request_device(&wgpu::DeviceDescriptor
    {
        extensions: wgpu::Extensions
        {
            anisotropic_filtering: false
        },
        limits: wgpu::Limits::default()
    });

    // Retrieve shader module
    let cs = include_bytes!("../spv/shader.comp.spv");
    let cs_module = device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&cs[..])).unwrap());

    // Copy image buffer to device
    let buffer = device.create_buffer_mapped(
            len as usize,
            wgpu::BufferUsage::MAP_WRITE
          | wgpu::BufferUsage::MAP_READ
          | wgpu::BufferUsage::COPY_SRC
          | wgpu::BufferUsage::COPY_DST
    ).fill_from_slice(&in_buf);

    // Setup texture dimensions
    let texture_extent = wgpu::Extent3d
    {
        width: width,
        height: height,
        depth: 1
    };

    // Create texture on GPU
    let texture = device.create_texture(&wgpu::TextureDescriptor
    {
        size: texture_extent,
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Uint,
        usage: wgpu::TextureUsage::SAMPLED
             | wgpu::TextureUsage::COPY_SRC
             | wgpu::TextureUsage::COPY_DST
    });

    // Generate default view for the texture
    let texture_view = texture.create_default_view();

    // Bind input texture to the shader
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        bindings: &[wgpu::BindGroupLayoutBinding
                    {
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::SampledTexture
                        {
                            multisampled: false,
                            dimension: wgpu::TextureViewDimension::D2
                        }
                    }]
    });

    // Setup bind group based on created layout
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor
    {
        layout: &bind_group_layout,
        bindings: &[wgpu::Binding
                    {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture_view)
                    }]
    });

    // Create pipeline for shader computations
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor
    {
        bind_group_layouts: &[&bind_group_layout]
    });

    // Setup pipeline based on created layout
    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor
    {
        layout: &pipeline_layout,
        compute_stage: wgpu::ProgrammableStageDescriptor
        {
            module: &cs_module,
            entry_point: "main",
        }
    });

    // Start recording time after fixed time costs above
    let now = Instant::now();

    // Create a command encoder for the GPU
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

    // Instruct the GPU to copy the image from its buffer to the texture
    encoder.copy_buffer_to_texture(
        wgpu::BufferCopyView
        {
            buffer: &buffer,
            offset: 0,
            row_pitch: width * std::mem::size_of::<u32>() as u32,
            image_height: height
        },
        wgpu::TextureCopyView
        {
            texture: &texture,
            mip_level: 0,
            array_layer: 0,
            origin: wgpu::Origin3d
            {
                x: 0.0,
                y: 0.0,
                z: 0.0
            }
        },
        texture_extent
    );

    // Run the computations on the image
    {
        let mut cpass = encoder.begin_compute_pass();
        cpass.set_pipeline(&compute_pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch(width, height, 1);
    }

    // Copy the output texture back to the buffer
    encoder.copy_texture_to_buffer(
        wgpu::TextureCopyView {
            texture: &texture,
            mip_level: 0,
            array_layer: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        wgpu::BufferCopyView {
            buffer: &buffer,
            offset: 0,
            row_pitch: width * std::mem::size_of::<u32>() as u32,
            image_height: height,
        },
        texture_extent,
    );

    // Execute the instructions outlined above
    queue.submit(&[encoder.finish()]);

    // Read the buffer to construct output image
    buffer.map_read_async(0, len, move | result: wgpu::BufferMapAsyncResult<&[u8]> |
    {
        if let Ok(mapping) = result
        {
            let out_buf: Vec<u8> = mapping.data.chunks_exact(1).map(|c| u8::from_ne_bytes([c [0]])).collect();
            let out = image::RgbaImage::from_raw(width, height, out_buf).unwrap();
            out.save(std::path::Path::new("output.bmp")).unwrap();
        }
    });

    // Record time after image copied back to CPU
    let elapsed = now.elapsed();
    let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
    println!("Image processed in {} seconds", sec);
}

fn main()
{
    futures::executor::block_on(run());
}

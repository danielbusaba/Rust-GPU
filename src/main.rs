async fn run()
{
    let args: Vec<String> = std::env::args().collect();
    let file: &str = &args[1];

    let image_file = image::open(std::path::Path::new(file)).unwrap().to_rgba();
    let (width, height) = image_file.dimensions();
    let len = (width * height * 4) as u64;
    let buf = image_file.into_raw();

    let adapter = wgpu::Adapter::request(
        &wgpu::RequestAdapterOptions
        {
            power_preference: wgpu::PowerPreference::HighPerformance,
            backends: wgpu::BackendBit::PRIMARY
        }
    ).unwrap();

    let (device, mut queue) = adapter.request_device(&wgpu::DeviceDescriptor
    {
        extensions: wgpu::Extensions
        {
            anisotropic_filtering: false
        },
        limits: wgpu::Limits::default()
    });

    let cs = include_bytes!("../spv/shader.comp.spv");
    let cs_module = device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&cs[..])).unwrap());

    let input_buffer = device.create_buffer_mapped(
            len as usize,
            wgpu::BufferUsage::MAP_WRITE
          | wgpu::BufferUsage::MAP_READ
          | wgpu::BufferUsage::COPY_DST
          | wgpu::BufferUsage::COPY_SRC
    ).fill_from_slice(&buf);

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        size: len,
        usage: wgpu::BufferUsage::MAP_READ
             | wgpu::BufferUsage::COPY_DST
             | wgpu::BufferUsage::COPY_SRC
    });

    let texture_extent = wgpu::Extent3d
    {
        width: width,
        height: height,
        depth: 1
    };

    let input_texture = device.create_texture(&wgpu::TextureDescriptor
    {
        size: texture_extent,
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Uint,
        usage: wgpu::TextureUsage::SAMPLED
             | wgpu::TextureUsage::COPY_DST
    });

    let input_texture_view = input_texture.create_default_view();

    // The output buffer lets us retrieve the data as an array
    let output_texture = device.create_texture(&wgpu::TextureDescriptor
    {
        size: texture_extent,
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Uint,
        usage: wgpu::TextureUsage::SAMPLED
             | wgpu::TextureUsage::COPY_SRC
    });

    let output_texture_view = output_texture.create_default_view();

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
                    },
                    wgpu::BindGroupLayoutBinding
                    {
                        binding: 1,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::SampledTexture
                        {
                            multisampled: false,
                            dimension: wgpu::TextureViewDimension::D2
                        }
                    }]
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor
    {
        layout: &bind_group_layout,
        bindings: &[wgpu::Binding
                    {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&input_texture_view)
                    },
                    wgpu::Binding
                    {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&output_texture_view)
                    }]
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor
    {
        bind_group_layouts: &[&bind_group_layout]
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor
    {
        layout: &pipeline_layout,
        compute_stage: wgpu::ProgrammableStageDescriptor
        {
            module: &cs_module,
            entry_point: "main",
        }
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

    encoder.copy_buffer_to_texture(
        wgpu::BufferCopyView
        {
            buffer: &input_buffer,
            offset: 0,
            row_pitch: width * std::mem::size_of::<u32>() as u32,
            image_height: height
        },
        wgpu::TextureCopyView
        {
            texture: &input_texture,
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

    {
        let mut cpass = encoder.begin_compute_pass();
        cpass.set_pipeline(&compute_pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch(width, height, 1);
    }

    encoder.copy_texture_to_buffer(
        wgpu::TextureCopyView {
            texture: &output_texture,
            mip_level: 0,
            array_layer: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        wgpu::BufferCopyView {
            buffer: &output_buffer,
            offset: 0,
            row_pitch: width * std::mem::size_of::<u32>() as u32,
            image_height: height,
        },
        texture_extent,
    );

    queue.submit(&[encoder.finish()]);

    output_buffer.map_read_async(0, len, move | result: wgpu::BufferMapAsyncResult<&[u8]> |
    {
        if let Ok(mapping) = result
        {
            let buffer: Vec<u8> = mapping.data.chunks_exact(1).map(|c| u8::from_ne_bytes([c [0]])).collect();
            let out = image::RgbaImage::from_raw(width, height, buffer).unwrap();
            out.save(std::path::Path::new("output.bmp")).unwrap();
        }
    });

    input_buffer.map_read_async(0, len, move | result: wgpu::BufferMapAsyncResult<&[u8]> |
    {
        if let Ok(mapping) = result
        {
            let buffer: Vec<u8> = mapping.data.chunks_exact(1).map(|c| u8::from_ne_bytes([c [0]])).collect();
            let out = image::RgbaImage::from_raw(width, height, buffer).unwrap();
            out.save(std::path::Path::new("input.bmp")).unwrap();
        }
    });
}

fn main()
{
    futures::executor::block_on(run());
}

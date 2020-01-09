async fn run()
{
    let image_file = image::open(std::path::Path::new("images/mdb001.pgm")).unwrap().to_rgb();    //DSC_0716.JPG mdb001.pgm
    let (width, height) = image_file.dimensions();
    let len = (width * height * 3) as u64;
    let buf = image_file.into_raw();

    let adapter = wgpu::Adapter::request(
        &wgpu::RequestAdapterOptions
        {
            power_preference: wgpu::PowerPreference::HighPerformance,
            backends: wgpu::BackendBit::PRIMARY
        }
    ).unwrap();

    let (device, mut queue) = adapter.request_device(&wgpu::DeviceDescriptor {
        extensions: wgpu::Extensions {
            anisotropic_filtering: false,
        },
        limits: wgpu::Limits::default(),
    });

    let cs = include_bytes!("../spv/shader.comp.spv");
    let cs_module =
        device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&cs[..])).unwrap());

    // The output buffer lets us retrieve the data as an array
    let input_buffer = device.create_buffer_mapped(
            len as usize,
            wgpu::BufferUsage::MAP_WRITE
            | wgpu::BufferUsage::MAP_READ
            | wgpu::BufferUsage::COPY_DST
            | wgpu::BufferUsage::COPY_SRC
            | wgpu::BufferUsage::UNIFORM
    ).fill_from_slice(&buf);

    // The output buffer lets us retrieve the data as an array
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        size: len,
        usage: wgpu::BufferUsage::MAP_READ
             | wgpu::BufferUsage::COPY_DST
             | wgpu::BufferUsage::COPY_SRC
             | wgpu::BufferUsage::UNIFORM
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        bindings: &[wgpu::BindGroupLayoutBinding
                    {
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::UniformBuffer
                        {
                            dynamic: false
                        }
                    },
                    wgpu::BindGroupLayoutBinding
                    {
                        binding: 1,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::UniformBuffer
                        {
                            dynamic: false
                        }
                    }]
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor
    {
        layout: &bind_group_layout,
        bindings: &[wgpu::Binding
                    {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &input_buffer,
                            range: 0 .. len,
                        },
                    },
                    wgpu::Binding
                    {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &output_buffer,
                            range: 0 .. len,
                        },
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
    encoder.copy_buffer_to_buffer(&output_buffer, 0, &output_buffer, 0, len);
    {
        let mut cpass = encoder.begin_compute_pass();
        cpass.set_pipeline(&compute_pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch(len as u32, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output_buffer, 0, &output_buffer, 0, len);

    queue.submit(&[encoder.finish()]);

    output_buffer.map_read_async(0, len, | result: wgpu::BufferMapAsyncResult<&[u8]> |
    {
        if let Ok(mapping) = result
        {
            let buffer: Vec<u8> = mapping.data.chunks_exact(1).map(|c| u8::from_ne_bytes([c [0]])).collect();
            let out = image::RgbImage::from_raw(1024, 1024, buffer).unwrap(); //6000 4000 1024 1024
            out.save(std::path::Path::new("images/mdb001_out.bmp")).unwrap();
        }
    });

    input_buffer.map_read_async(0, len, | result: wgpu::BufferMapAsyncResult<&[u8]> |
    {
        if let Ok(mapping) = result
        {
            let buffer: Vec<u8> = mapping.data.chunks_exact(1).map(|c| u8::from_ne_bytes([c [0]])).collect();
            let out = image::RgbImage::from_raw(1024, 1024, buffer).unwrap();
            out.save(std::path::Path::new("images/mdb001_in.bmp")).unwrap();
        }
    });
}

fn main() {
    futures::executor::block_on(run());
}

use std;
use std::{io, time::Instant, process::exit, process::ExitCode};
use ffmpeg_next;
use crossterm;
use rodio;

const CHRLIST: &str = "    ```...---'':::___,,,^^===;;;>>><<<++!!!rrrccc**///zzz???ssLLLTTTvvv)))JJ777(((|||FFiii{{{CCC}}}ffIII333111ttllluuu[[[nneeeoooZZZ555YYxxxjjjyyyaa]]]222EEESSwwwqqqkkkPPP66hhh999ddd44VVVpppOOOGGGbbUUUAAAKKKXXHHHmmm888RRDDD###$$$BBBgg000MMMNNNWWQQQ%%%&&&@@";

fn main() -> ExitCode {
    ffmpeg_next::init().expect("your mom");

    let input_file_name = std::env::args().nth(1);
    if input_file_name == None {
        eprintln!("no input file you dummy");
        return ExitCode::from(69);
    }
    let input_file_name = input_file_name.unwrap();
    
    let mut no_color = false;
    {
        let an_opt = std::env::args().nth(2);

        if an_opt != None {
            let an_opt = an_opt.unwrap();
            if an_opt == "-nc" || an_opt == "--no-color" {
                no_color = true;
            }
            else {
                eprintln!("\"{}\" is not an option", an_opt);
                return ExitCode::from(69);
            }
        }
    }
    
    let mut teto = ffmpeg_next::format::input(&input_file_name).inspect_err(|e| {
        eprintln!("can't open this file because: {e}");
        exit(67);
    }).unwrap();


    let the_video_stream = teto.streams().best(ffmpeg_next::media::Type::Video).ok_or(0).inspect_err(|_| {
        eprintln!("ffmpeg can open this file but it's not a video!");
        exit(67);
    }).unwrap();
    let the_video_stream_index = the_video_stream.index();
    
    let mut sis = ffmpeg_next::codec::context::Context::from_parameters(the_video_stream.parameters()).expect("um").decoder().video().expect("ummmm");
    

    let the_audio_stream = teto.streams().best(ffmpeg_next::media::Type::Audio).ok_or(0).inspect_err(|_| {
        eprintln!("a video with no audio isn't very fun");
        exit(67);
    }).unwrap();
    let the_audio_stream_index = the_audio_stream.index();

    let mut bro = ffmpeg_next::codec::context::Context::from_parameters(the_audio_stream.parameters()).expect("um").decoder().audio().expect("ummmm");

    let mut sink = rodio::DeviceSinkBuilder::open_default_sink().expect("let that sink in");
    sink.log_on_drop(false);
    let water = rodio::Player::connect_new(&sink.mixer());


    let seconds_per_frame = 1f64 / f64::from(the_video_stream.rate());
    let mut remember_color: u32 = 0;
    let mut skip = 0;
    let mut video_frames_done = 0;
    let mut video_done = false;
    let mut audio_done = false;

    let mut timer = Instant::now();

    for (stream, packet) in teto.packets() {
        if stream.index() == the_video_stream_index {
            sis.send_packet(&packet).expect("nuh");
            
            let mut decoded = ffmpeg_next::frame::Video::empty();
            while sis.receive_frame(&mut decoded).is_ok() {
                if skip > 0 {
                    skip -= 1;
                    continue;
                }

                let (outw, outh) = crossterm::terminal::size().expect("e");
                let mut scaler = ffmpeg_next::software::scaling::context::Context::get(
                    sis.format(), sis.width(), sis.height(), 
                    ffmpeg_next::format::Pixel::RGB24, outw as u32, outh as u32, 
                    ffmpeg_next::software::scaling::Flags::BILINEAR
                ).expect("no");

                let mut frame = ffmpeg_next::frame::Video::empty();
                scaler.run(&decoded, &mut frame).expect("aaaaaaaaaaaaaaaa");

                crossterm::execute!(io::stdout(), crossterm::cursor::MoveTo(0,0)).unwrap();

                for y in 0..frame.height() {
                    for x in 0..frame.width() {
                        let r = frame.data(0)[0 + ((x * 3) + y * (frame.stride(0) as u32)) as usize];
                        let g = frame.data(0)[1 + ((x * 3) + y * (frame.stride(0) as u32)) as usize];
                        let b = frame.data(0)[2 + ((x * 3) + y * (frame.stride(0) as u32)) as usize];
                        
                        if !no_color {
                            let chroma = u32::from_be_bytes([r,g,b,0]);
    
                            if chroma != remember_color {
                                crossterm::execute!(io::stdout(), 
                                    crossterm::style::SetForegroundColor(crossterm::style::Color::Rgb { r, g, b })
                                ).unwrap();
                            }
                            remember_color = chroma;
                        }
                        
                        let luma = (
                            r as f64 * 0.299 +
                            g as f64 * 0.587 +
                            b as f64 * 0.114
                        ) as u8;
                        
                        let luma_char = CHRLIST.chars().nth(luma as usize).unwrap();
                        print!("{}", luma_char);
                    }
                }

                video_frames_done += 1;
            }
            
            video_done = true;
        }
        else
        if stream.index() == the_audio_stream_index {
            let mut resampler = ffmpeg_next::software::resampling::context::Context::get(
                bro.format(), bro.channel_layout(), bro.rate(), 
                ffmpeg_next::format::Sample::F32(ffmpeg_next::format::sample::Type::Planar),
                ffmpeg_next::channel_layout::ChannelLayout::default(1), bro.rate(),
            ).expect("no");

            let mut chunk = vec![];

            bro.send_packet(&packet).expect("nuh");

            let mut decoded = ffmpeg_next::frame::Audio::empty();
            while bro.receive_frame(&mut decoded).is_ok() {
                let mut frame = ffmpeg_next::frame::Audio::empty();
                resampler.run(&decoded, &mut frame).expect("aaaaaaaaaaaaaaaa");

                chunk = Vec::with_capacity(frame.samples());
                for i in 0..frame.samples() {
                    let n = frame.format().bytes();
                    assert_eq!(n, 4, "an f32 should probably be 4 bytes");
                    chunk.push(f32::from_ne_bytes(frame.data(0)[i * n .. i * n + n].try_into().expect("e")));
                }
            }

            if !chunk.is_empty() {
                let nb_channels = resampler.output().channel_layout.0.nb_channels as u16;
                let sample_rate = resampler.output().rate / nb_channels as u32;
                water.append(rodio::buffer::SamplesBuffer::new(
                    std::num::NonZeroU16::new(nb_channels).expect("oh my god! this is terrible! how could this value be Zero! this is horrible!"),
                    std::num::NonZeroU32::new(sample_rate).expect("oh my god! this is terrible! how could this value be Zero! this is horrible!"),
                    chunk
                ));
            }

            audio_done = true;
        }
        
        if video_done && audio_done {
            while timer.elapsed().as_secs_f64() < (seconds_per_frame * video_frames_done as f64) {};
            let t = timer.elapsed().as_secs_f64();
            skip = (t / seconds_per_frame) as i32 - video_frames_done;

            timer = Instant::now();
            
            video_frames_done = 0;
            video_done = false;
            audio_done = false;
        }
    }
    
    sis.send_eof().unwrap();
    bro.send_eof().unwrap();

    crossterm::execute!(io::stdout(), 
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        crossterm::style::ResetColor,
        crossterm::cursor::MoveTo(0,0)
    ).unwrap();
    
    println!("goodbye!");
    
    return ExitCode::SUCCESS;
}

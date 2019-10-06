/*
 * The MIT License (MIT)
 *
 * Copyright (c) 2015 Andres Vahter (andres.vahter@gmail.com)
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

// import local modules
extern crate demod_fm;
use demod_fm::usage;
use demod_fm::usage::DataType::{S8,U8,I16, F32};

// import external modules
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::BufWriter;
use std::mem;
use std::slice;

extern crate liquid_dsp;
use liquid_dsp::firfilt;
use liquid_dsp::msresamp;
use liquid_dsp::freqdem;

extern crate num;
use num::complex::Complex;

const BUFFER_SIZE: usize = 8192;

macro_rules! println_stderr(
    ($($arg:tt)*) => (
        match writeln!(&mut ::std::io::stderr(), $($arg)* ) {
            Ok(_) => {},
            Err(x) => panic!("Unable to write to stderr: {}", x),
        }
    )
);

fn main() {
    let args = usage::args();

    println_stderr!("demod_fm {} andres.vahter@gmail.com\n\n", env!("CARGO_PKG_VERSION"));

    // filter options
    let filter_len = 64;
    let filter_cutoff_freq = args.bandwidth.unwrap() as f32 / args.samplerate.unwrap() as f32;
    let filter_attenuation = 70.0f32;

    let filter = firfilt::FirFilterCrcf::kaiser(filter_len, filter_cutoff_freq, filter_attenuation, 0.0f32);
    filter.set_scale(2.0f32 * filter_cutoff_freq);


    // resampler options
    let resampler_rate = if args.resamplerate.is_some() {
                            args.resamplerate.unwrap() as f32 / args.samplerate.unwrap() as f32
                        }
                        else {
                            1.0_f32
                        };

    let resampler = msresamp::MsresampCrcf::new(resampler_rate, filter_attenuation);

    let num_samples = match args.inputtype.unwrap() {
        S8 => {BUFFER_SIZE as u32 / 2},
        U8 => {BUFFER_SIZE as u32 / 2},
        I16 => {BUFFER_SIZE as u32 / 4},
        F32 => {BUFFER_SIZE as u32 / 8},
    };

    let mut input = vec![Complex::<f32>::new(0.0f32, 0.0f32); num_samples as usize];

    // FM demodulator
    let modulation_factor = if args.resamplerate.is_some() {
                                args.fmargs.deviation.unwrap() as f32 / args.resamplerate.unwrap() as f32
                            }
                            else {
                                args.fmargs.deviation.unwrap() as f32 / args.samplerate.unwrap() as f32
                            };

    let fm_demod = freqdem::Freqdem::new(modulation_factor);

    let mut stdin = BufReader::with_capacity(BUFFER_SIZE*2, io::stdin());
    let mut stdout = BufWriter::new(io::stdout());

    loop {
        let invec = stdin.by_ref().bytes().take(BUFFER_SIZE).collect::<Result<Vec<u8>,_>>().ok().expect("doppler collect error");
        let mut sample_count: usize = 0;

        match args.inputtype.unwrap() {
            S8 => {
                for b in invec.chunks(2) {
                    let i: f32 = 2. * ((b[0] as i8) as f32 + 128.) / 255. - 1.;
                    let q: f32 = 2. * ((b[1] as i8) as f32 + 128.) / 255. - 1.;

                    input[sample_count] = Complex::<f32>::new(i, q);
                    sample_count += 1;
                }
            }

            U8 => {
                for b in invec.chunks(2) {
                    let i: f32 = 2. * ((b[0] as u8 ) as f32 / 255.) - 1.;
                    let q: f32 = 2. * ((b[1] as u8 ) as f32 / 255.) - 1.;

                    input[sample_count] = Complex::<f32>::new(i, q);
                    sample_count += 1;
                }
            }

            I16 => {
                for b in invec.chunks(4) {
                    let i: f32 = ((b[1] as i16) << 8 | b[0] as i16) as f32 / 32768.;
                    let q: f32 = ((b[3] as i16) << 8 | b[2] as i16) as f32 / 32768.;

                    input[sample_count] = Complex::<f32>::new(i, q);
                    sample_count += 1;
                }
            }
            F32 => {
                for b in invec.chunks(8) {
                    let i: f32 = unsafe {mem::transmute::<u32, f32>(((b[3] as u32) << 24) | ((b[2] as u32) << 16) | ((b[1] as u32) << 8) | b[0] as u32)};
                    let q: f32 = unsafe {mem::transmute::<u32, f32>(((b[7] as u32) << 24) | ((b[6] as u32) << 16) | ((b[5] as u32) << 8) | b[4] as u32)};

                    input[sample_count] = Complex::<f32>::new(i, q);
                    sample_count += 1;
                }
            }
        }

        // filter
        filter.execute_block(&mut input[0..sample_count]);

        // resample
        let resampler_output = resampler.resample(&mut input[0..sample_count]);
        let resampler_output_len = resampler_output.len();

        // demodulate
        let mut demod_f32_out = fm_demod.demodulate_block(&resampler_output);

        match args.outputtype.unwrap() {
            S8 => {
                let mut demod_s8_out = vec![0_i8; resampler_output_len];
                for i in 0 .. resampler_output_len {
                    if args.fmargs.squarewave.unwrap() {
                        // make output square like, multimon-ng likes it more
                        if demod_f32_out[i] > 0.0 {
                            demod_f32_out[i] = 1.0;
                        }
                        if demod_f32_out[i] < 0.0 {
                            demod_f32_out[i] = -1.0;
                        }
                    }
                    else {
                        // clamp output
                        if demod_f32_out[i] > 1.0 {
                            demod_f32_out[i] = 1.0;
                        }
                        if demod_f32_out[i] < -1.0 {
                            demod_f32_out[i] = -1.0;
                        }
                    }

                    demod_s8_out[i] = (255. * ((demod_f32_out[i] + 1.) / 2.) - 128. ) as i8;
                }

                let slice = unsafe {slice::from_raw_parts(demod_s8_out.as_ptr() as *const _, resampler_output_len as usize)};
                stdout.write(&slice).map_err(|e|{println_stderr!("demod stdout.write error: {}", e);}).unwrap();
                stdout.flush().map_err(|e|{println_stderr!("demod stdout.flush error: {}", e);}).unwrap();
            }

            U8 => {
                let mut demod_u8_out = vec![0_u8; resampler_output_len];
                for i in 0 .. resampler_output_len {
                    if args.fmargs.squarewave.unwrap() {
                        // make output square like, multimon-ng likes it more
                        if demod_f32_out[i] > 0.0 {
                            demod_f32_out[i] = 1.0;
                        }
                        if demod_f32_out[i] < 0.0 {
                            demod_f32_out[i] = -1.0;
                        }
                    }
                    else {
                        // clamp output
                        if demod_f32_out[i] > 1.0 {
                            demod_f32_out[i] = 1.0;
                        }
                        if demod_f32_out[i] < -1.0 {
                            demod_f32_out[i] = -1.0;
                        }
                    }

                    demod_u8_out[i] = 255 * ((demod_f32_out[i] + 1.) / 2.) as u8;
                }

                let slice = unsafe {slice::from_raw_parts(demod_u8_out.as_ptr() as *const _, resampler_output_len as usize)};
                stdout.write(&slice).map_err(|e|{println_stderr!("demod stdout.write error: {}", e);}).unwrap();
                stdout.flush().map_err(|e|{println_stderr!("demod stdout.flush error: {}", e);}).unwrap();
            }

            I16 => {
                let mut demod_i16_out = vec![0_i16; resampler_output_len];
                for i in 0 .. resampler_output_len {
                    if args.fmargs.squarewave.unwrap() {
                        // make output square like, multimon-ng likes it more
                        if demod_f32_out[i] > 0.0 {
                            demod_f32_out[i] = 1.0;
                        }
                        if demod_f32_out[i] < 0.0 {
                            demod_f32_out[i] = -1.0;
                        }
                    }
                    else {
                        // clamp output
                        if demod_f32_out[i] > 1.0 {
                            demod_f32_out[i] = 1.0;
                        }
                        if demod_f32_out[i] < -1.0 {
                            demod_f32_out[i] = -1.0;
                        }
                    }

                    demod_i16_out[i] = (demod_f32_out[i] * 32767_f32) as i16;
                }

                let slice = unsafe {slice::from_raw_parts(demod_i16_out.as_ptr() as *const _, (resampler_output_len * 2) as usize)};
                stdout.write(&slice).map_err(|e|{println_stderr!("demod stdout.write error: {}", e);}).unwrap();
                stdout.flush().map_err(|e|{println_stderr!("demod stdout.flush error: {}", e);}).unwrap();
            }
            F32 => {
                let slice = unsafe {slice::from_raw_parts(demod_f32_out.as_ptr() as *const _, (resampler_output_len * 4) as usize)};
                stdout.write(&slice).map_err(|e|{println_stderr!("stdout.write error: {}", e);}).unwrap();
                stdout.flush().map_err(|e|{println_stderr!("stdout.flush error: {}", e);}).unwrap();
            }
        }

        if invec.len() != BUFFER_SIZE {
            break;
        }
    }
}

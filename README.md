# demod_fm
Command line utility based on liquid-dsp for demodulating SDR IQ streams.
Firstly it was written in C ([last commit to C version](https://github.com/cubehub/demod/commit/1b1736ec72adc5b36db951be41dceaf3badccea9)), however now it is rewritten in [rust](http://www.rust-lang.org).

## what the fork?

#### Additions

This fork adds the input and output support of:

- unsigned 8 bits integer (u8)
- signed 8 bits integer (s8)

u8 input format is useful to work with [**rtl_sdr**](https://github.com/osmocom/rtl-sdr) recorded IQ files while s8 input format will be necessary to work with [**hackrf_transfer**](https://github.com/mossmann/hackrf) IQ recorded files.

## dependencies

#### autoconf
    sudo apt-get install autoconf

#### [liquid-dsp](https://github.com/jgaeddert/liquid-dsp)
    git clone git://github.com/jgaeddert/liquid-dsp.git
    cd liquid-dsp
    ./bootstrap.sh
    ./configure
    make
    sudo make install
    sudo ldconfig


#### rust
http://www.rust-lang.org/install.html

    curl https://sh.rustup.rs -sSf | sh

## build

    git clone https://github.com/cdeletre/demod_fm.git
    cd demod_fm
    cargo build --release

## install
#### mac os x

    cp target/release/demod_fm /usr/local/bin/

#### linux

    sudo cp target/release/demod_fm /usr/local/bin/

## usage

#### play FM radio recording

Play FM radio recording (deemph filter not used and __does not play in stereo__)

    cat testfiles/fm_radio_rds_228000sps_s8.iq | demod_fm --samplerate 228000 --intype s8 --outtype i16 --bandwidth 100000 fm --deviation 75000 | play -t raw -r 228000 -e signed-integer -b16 -c 1 -V1 -
    cat testfiles/fm_radio_rds_228000sps_u8.iq | demod_fm --samplerate 228000 --intype u8 --outtype i16 --bandwidth 100000 fm --deviation 75000 | play -t raw -r 228000 -e signed-integer -b16 -c 1 -V1 -
    cat testfiles/fm_radio_rds_228000sps_s16.iq | demod_fm --samplerate 228000 --intype i16 --outtype i16 --bandwidth 100000 fm --deviation 75000 | play -t raw -r 228000 -e signed-integer -b16 -c 1 -V1 -
    cat testfiles/fm_radio_rds_228000sps_f32.iq | demod_fm --samplerate 228000 --intype f32 --outtype f32 --bandwidth 100000 fm --deviation 75000 | play -t raw -r 228000 -e floating-point -b32 -c 1 -V1 -

#### demodulate FSK9600 wav IQ data recording

Demodulate FSK9600 raw IQ data recording and pipe output to multimon-ng for packet decoding, notice `--squarewave` flag is added to FM demodulation, which makes demodulator output square like (multimon-ng likes it more)

    sox -t wav sdr_fsk9600.wav -esigned-integer -b16  -r 126000 -t raw - | demod_fm --samplerate 126000 --resamplerate 48000 --bandwidth 4500 fm --deviation 3500 --squarewave | multimon-ng -t raw -a FSK9600 /dev/stdin

for testing AX25 decoding use this [ax25_fsk9600_1024k_i16.wav](https://github.com/cubehub/samples/blob/master/ax25_fsk9600_1024k_i16.wav) with the following command (install `doppler` from [here](https://github.com/cubehub/doppler)):

    sox -t wav ax25_fsk9600_1024k_i16.wav -esigned-integer -b16  -r 126000 -t raw - | doppler const -s 126000 -i i16 --shift 14500 | demod_fm -s 126000 -r 48000 -i i16 -o i16 --bandwidth 4500 fm --deviation 3500 --squarewave | multimon-ng -t raw -a FSK9600 /dev/stdin

Notice that here [modified multimon-ng](https://github.com/cubehub/multimon-ng) is used that supports 48000 sps input stream for fsk9600 decoder. Read [here](http://andres.svbtle.com/pipe-sdr-iq-data-through-fm-demodulator-for-fsk9600-ax25-reception) why multimon-ng must be modified instead of converting **demod** output to native 22050 format.

#### decode RDS data

Decode RDS data easily by piping to [redsea](https://github.com/windytan/redsea) tool.

	cat testfiles/fm_radio_rds_228000sps_u8.iq | demod_fm --samplerate 228000 --intype u8 --outtype i16 --bandwidth 100000 fm --deviation 75000 | redsea  -r 228000

#### test files

An IQ record of FM radio with RDS data has been generated from a [Stereo MPX signal](https://en.wikipedia.org/wiki/FM_broadcasting#Stereo_FM) produced with [**PiFMRds**](https://github.com/ChristopheJacquet/PiFmRds) and then modulated with the **GNU Radio** [**Frequency Mod**](https://wiki.gnuradio.org/index.php/Frequency_Mod) block:

	testfiles/fm_radio_rds_228000sps_f32.iq

The resulting IQ record has also been resampled from its original *32 bits float complex* format to *8 bits signed integer complex*, *8 bits unsigned integer complex* and *16 bits signed integer complex*:

	testfiles/fm_radio_rds_228000sps_s8.iq
	testfiles/fm_radio_rds_228000sps_u8.iq
	testfiles/fm_radio_rds_228000sps_s16.iq
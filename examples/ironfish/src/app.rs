use crate::{
    makepad_widgets::*,
    makepad_audio_graph::*,
    makepad_platform::midi::*,
    
    makepad_synth_ironfish::ironfish::*,
    makepad_audio_widgets::piano::*,
    sequencer::*,
    makepad_audio_widgets::display_audio::*
};

//use std::fs::File;
//use std::io::prelude::*;
live_design!{
    import makepad_widgets::frame::*
    import makepad_example_ironfish::app_desktop::AppDesktop
    import makepad_example_ironfish::app_mobile::AppMobile
    import makepad_widgets::desktop_window::DesktopWindow
    
    import makepad_audio_graph::mixer::Mixer;
    import makepad_audio_graph::instrument::Instrument;
    import makepad_synth_ironfish::ironfish::IronFish;
    
    // APP
    App = {{App}} {
        audio_graph: {
            root: <Mixer> {
                c1 = <Instrument> {
                    <IronFish> {}
                }
            }
        }
        
        //ui: <AppMobile> {}
        ui: <Frame> {
            <DesktopWindow> {
                window: {inner_size: vec2(1280, 1000)},
                pass: {clear_color: #2A}
                frame: {body = {
                    <AppDesktop> {}
                }}
            }
            <DesktopWindow> {
                window: {inner_size: vec2(400, 600)},
                pass: {clear_color: #2A}
                frame: {body = {
                    <AppMobile> {}
                }}
            }
        }
    }
}
app_main!(App);

#[derive(Live)]
#[live_design_with {
    crate::makepad_audio_widgets::live_design(cx);
    crate::makepad_audio_graph::live_design(cx);
    crate::makepad_synth_ironfish::live_design(cx);
    crate::sequencer::live_design(cx);
    crate::app_desktop::live_design(cx);
    crate::app_mobile::live_design(cx);
}]
pub struct App {
    ui: WidgetRef,
    audio_graph: AudioGraph,
    #[rust] midi_input: MidiInput,
}

impl LiveHook for App {
    fn before_apply(&mut self, _cx: &mut Cx, _apply_from: ApplyFrom, _index: usize, _nodes: &[LiveNode]) -> Option<usize> {
        //_nodes.debug_print(0,100);
        None
    }
}

impl App {
    
    pub fn data_bind(&mut self, cx: &mut Cx, db: &mut DataBinding, actions: &WidgetActions) {
        let mut db = db.borrow_cx(cx, &self.ui, actions);
        // touch
        data_to_widget!(db, touch.scale => touch.scale.slider);
        data_to_widget!(db, touch.scale => touch.scale.slider);
        data_to_widget!(db, touch.curve => touch.curve.slider);
        data_to_widget!(db, touch.offset => touch.offset.slider);
        data_to_widget!(db, filter1.touch_amount => touch.touchamount.slider);
        
        // sequencer
        data_to_widget!(db, sequencer.playing => playpause.checkbox);
        data_to_widget!(db, sequencer.bpm => speed.slider);
        data_to_widget!(db, sequencer.rootnote => rootnote.dropdown);
        data_to_widget!(db, sequencer.scale => scaletype.dropdown);
        data_to_widget!(db, arp.enabled => arp.checkbox);
        data_to_widget!(db, arp.octaves => arpoctaves.slider);
        
        // Mixer panel
        data_to_widget!(db, osc_balance => balance.slider);
        data_to_widget!(db, noise => noise.slider);
        data_to_widget!(db, sub_osc => sub.slider);
        data_to_widget!(db, portamento => porta.slider);
        
        // DelayFX Panel
        data_to_widget!(db, delay.delaysend => delaysend.slider);
        data_to_widget!(db, delay.delayfeedback => delayfeedback.slider);
        
        data_to_widget!(db, bitcrush.enable => crushenable.checkbox);
        data_to_widget!(db, bitcrush.amount => crushamount.slider);
        
        data_to_widget!(db, delay.difference => delaydifference.slider);
        data_to_widget!(db, delay.cross => delaycross.slider);
        
        // Chorus panel
        data_to_widget!(db, chorus.mix => chorusmix.slider);
        data_to_widget!(db, chorus.mindelay => chorusdelay.slider);
        data_to_widget!(db, chorus.moddepth => chorusmod.slider);
        data_to_widget!(db, chorus.rate => chorusrate.slider);
        data_to_widget!(db, chorus.phasediff => chorusphase.slider);
        data_to_widget!(db, chorus.feedback => chorusfeedback.slider);
        
        // Reverb panel
        data_to_widget!(db, reverb.mix => reverbmix.slider);
        data_to_widget!(db, reverb.feedback => reverbfeedback.slider);
        
        //LFO Panel
        data_to_widget!(db, lfo.rate => rate.slider);
        data_to_widget!(db, filter1.lfo_amount => lfoamount.slider);
        data_to_widget!(db, lfo.synconkey => sync.checkbox);
        
        //Volume Envelope
        data_to_widget!(db, volume_envelope.a => vol_env.attack.slider);
        data_to_widget!(db, volume_envelope.h => vol_env.hold.slider);
        data_to_widget!(db, volume_envelope.d => vol_env.decay.slider);
        data_to_widget!(db, volume_envelope.s => vol_env.sustain.slider);
        data_to_widget!(db, volume_envelope.r => vol_env.release.slider);
        
        //Mod Envelope
        data_to_widget!(db, mod_envelope.a => mod_env.attack.slider);
        data_to_widget!(db, mod_envelope.h => mod_env.hold.slider);
        data_to_widget!(db, mod_envelope.d => mod_env.decay.slider);
        data_to_widget!(db, mod_envelope.s => mod_env.sustain.slider);
        data_to_widget!(db, mod_envelope.r => mod_env.release.slider);
        data_to_widget!(db, filter1.envelope_amount => modamount.slider);
        
        // Filter panel
        data_to_widget!(db, filter1.filter_type => filter_type.dropdown);
        data_to_widget!(db, filter1.cutoff => cutoff.slider);
        data_to_widget!(db, filter1.resonance => resonance.slider);
        
        // Osc1 panel
        data_to_widget!(db, supersaw1.spread => osc1.supersaw.spread.slider);
        data_to_widget!(db, supersaw1.diffuse => osc1.supersaw.diffuse.slider);
        data_to_widget!(db, supersaw1.spread => osc1.supersaw.spread.slider);
        data_to_widget!(db, supersaw1.diffuse => osc1.supersaw.diffuse.slider);
        data_to_widget!(db, supersaw1.spread => osc1.hypersaw.spread.slider);
        data_to_widget!(db, supersaw1.diffuse => osc1.hypersaw.diffuse.slider);
        
        data_to_widget!(db, osc1.osc_type => osc1.type.dropdown);
        data_to_widget!(db, osc1.transpose => osc1.transpose.slider);
        data_to_widget!(db, osc1.detune => osc1.detune.slider);
        data_to_widget!(db, osc1.harmonic => osc1.harmonicshift.slider);
        data_to_widget!(db, osc1.harmonicenv => osc1.harmonicenv.slider);
        data_to_widget!(db, osc1.harmoniclfo => osc1.harmoniclfo.slider);
        
        // Osc2 panel
        data_to_widget!(db, supersaw1.spread => osc2.supersaw.spread.slider);
        data_to_widget!(db, supersaw1.diffuse => osc2.supersaw.diffuse.slider);
        data_to_widget!(db, supersaw2.spread => osc2.supersaw.spread.slider);
        data_to_widget!(db, supersaw2.diffuse => osc2.supersaw.diffuse.slider);
        data_to_widget!(db, supersaw2.spread => osc2.hypersaw.spread.slider);
        data_to_widget!(db, supersaw2.diffuse => osc2.hypersaw.diffuse.slider);
        
        data_to_widget!(db, osc2.osc_type => osc2.type.dropdown);
        data_to_widget!(db, osc2.transpose => osc2.transpose.slider);
        data_to_widget!(db, osc2.detune => osc2.detune.slider);
        data_to_widget!(db, osc2.harmonic => osc2.harmonicshift.slider);
        data_to_widget!(db, osc2.harmonicenv => osc2.harmonicenv.slider);
        data_to_widget!(db, osc2.harmoniclfo => osc2.harmoniclfo.slider);
        
        // sequencer
        data_to_widget!(db, sequencer.steps => sequencer);
        
        data_to_apply!(db, osc1.osc_type => osc1.supersaw, visible => | v | v == id!(SuperSaw).to_enum());
        data_to_apply!(db, osc2.osc_type => osc2.supersaw, visible => | v | v == id!(SuperSaw).to_enum());
        data_to_apply!(db, osc1.osc_type => osc1.hypersaw, visible => | v | v == id!(HyperSaw).to_enum());
        data_to_apply!(db, osc2.osc_type => osc2.hypersaw, visible => | v | v == id!(HyperSaw).to_enum());
        data_to_apply!(db, osc1.osc_type => osc1.harmonic, visible => | v | v == id!(HarmonicSeries).to_enum());
        data_to_apply!(db, osc2.osc_type => osc2.harmonic, visible => | v | v == id!(HarmonicSeries).to_enum());
        
        data_to_apply!(db, mod_envelope.a => mod_env.display, draw_bg.attack => | v | v);
        data_to_apply!(db, mod_envelope.h => mod_env.display, draw_bg.hold => | v | v);
        data_to_apply!(db, mod_envelope.d => mod_env.display, draw_bg.decay => | v | v);
        data_to_apply!(db, mod_envelope.s => mod_env.display, draw_bg.sustain => | v | v);
        data_to_apply!(db, mod_envelope.r => mod_env.display, draw_bg.release => | v | v);
        data_to_apply!(db, volume_envelope.a => vol_env.display, draw_bg.attack => | v | v);
        data_to_apply!(db, volume_envelope.h => vol_env.display, draw_bg.hold => | v | v);
        data_to_apply!(db, volume_envelope.d => vol_env.display, draw_bg.decay => | v | v);
        data_to_apply!(db, volume_envelope.s => vol_env.display, draw_bg.sustain => | v | v);
        data_to_apply!(db, volume_envelope.r => vol_env.display, draw_bg.release => | v | v);
    }
    
    pub fn draw(&mut self, cx: &mut Cx2d) {
        while self.ui.draw_widget(cx).is_not_done() {};
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        
        if let Event::Draw(event) = event {
            return self.draw(&mut Cx2d::new(cx, event));
        }
        
        let ui = self.ui.clone();
        let mut db = DataBinding::new();
        
        let actions = ui.handle_widget_event(cx, event);
        
        if let Event::Construct = event {
            log!("{}", std::mem::size_of::<LiveValue>());
            let ironfish = self.audio_graph.by_type::<IronFish>().unwrap();
            db.to_widgets(ironfish.settings.live_read());
            ui.get_piano(id!(piano)).set_key_focus(cx);
            self.midi_input = cx.midi_input();
            //self.midi_data = cx.midi_output_create_sender();
        }
        
        if let Event::MidiPorts(ports) = event {
            log!("{}", ports);
            cx.use_midi_inputs(&ports.all_inputs());
        }
        
        if let Event::AudioDevices(devices) = event {
            //log!("{}", devices);
            cx.use_audio_outputs(&devices.default_output());
        }
        
        // ui.get_radio_group(&[
        //     id!(envelopes.tab1),
        //     id!(envelopes.tab2),
        // ]).selected_to_visible(cx, &ui, &actions, &[
        //     id!(envelopes.tab1_frame),
        //     id!(envelopes.tab2_frame),
        // ]);
        
        ui.get_radio_buttons(&[
            id!(oscillators.tab1),
            id!(oscillators.tab2),
        ]).selected_to_visible(cx, &ui, &actions, &[
            id!(oscillators.osc1),
            id!(oscillators.osc2),
        ]);
        
        ui.get_radio_buttons(&[
            id!(mobile_modes.tab1),
            id!(mobile_modes.tab2),
            id!(mobile_modes.tab3),
        ]).selected_to_visible(cx, &ui, &actions, &[
            id!(application_pages.tab1_frame),
            id!(application_pages.tab2_frame),
            id!(application_pages.tab3_frame),
        ]);
        
        let display_audio = ui.get_display_audio(id!(display_audio));
        
        let mut buffers = 0;
        self.audio_graph.handle_event_with(cx, event, &mut | cx, action | {
            match action {
                AudioGraphAction::DisplayAudio {buffer, voice, ..} => {
                    display_audio.process_buffer(cx, None, voice, buffer);
                    buffers += 1;
                }
                AudioGraphAction::VoiceOff {voice} => {
                    display_audio.voice_off(cx, voice);
                }
            };
        });
        
        let piano = ui.get_piano(id!(piano));
        
        while let Some((_, data)) = self.midi_input.receive() {
            self.audio_graph.send_midi_data(data);
            if let Some(note) = data.decode().on_note() {
                piano.set_note(cx, note.is_on, note.note_number)
            }
        }
        
        for note in piano.notes_played(&actions) {
            self.audio_graph.send_midi_data(MidiNote {
                channel: 0,
                is_on: note.is_on,
                note_number: note.note_number,
                velocity: note.velocity
            }.into());
        }
        
        if ui.get_button(id!(panic)).clicked(&actions) {
            cx.midi_reset();
            self.audio_graph.all_notes_off();
        }
        
        let sequencer = ui.get_sequencer(id!(sequencer));
        // lets fetch and update the tick.
        
        if ui.get_button(id!(clear_grid)).clicked(&actions) {
            sequencer.clear_grid(cx, &mut db);
        }
        
        if ui.get_button(id!(grid_down)).clicked(&actions) {
            sequencer.grid_down(cx, &mut db);
        }
        
        if ui.get_button(id!(grid_up)).clicked(&actions) {
            sequencer.grid_up(cx, &mut db);
        }
        
        self.data_bind(cx, &mut db, &actions);
        if let Some(nodes) = db.from_widgets() {
            let ironfish = self.audio_graph.by_type::<IronFish>().unwrap();
            ironfish.settings.apply_over(cx, &nodes);
        }
    }
    /*
    pub fn preset(&mut self, cx: &mut Cx, index: usize, save: bool) {
        let ironfish = self.audio_graph.by_type::<IronFish>().unwrap();
        let file_name = format!("preset_{}.txt", index);
        if save {
            let nodes = ironfish.settings.live_read();
            let data = nodes.to_cbor(0).unwrap();
            let data = makepad_miniz::compress_to_vec(&data, 10);
            let data = makepad_base64::base64_encode(&data, &makepad_base64::BASE64_URL_SAFE);
            log!("Saving preset {}", file_name);
            let mut file = File::create(&file_name).unwrap();
            file.write_all(&data).unwrap();
        }
        else if let Ok(mut file) = File::open(&file_name) {
            log!("Loading preset {}", file_name);
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if let Ok(data) = makepad_base64::base64_decode(&data) {
                if let Ok(data) = makepad_miniz::decompress_to_vec(&data) {
                    let mut nodes = Vec::new();
                    nodes.from_cbor(&data).unwrap();
                    ironfish.settings.apply_over(cx, &nodes);
                    //self.imgui.root_frame().bind_read(cx, &nodes);
                }
                else {
                    log!("Error decompressing preset");
                }
            }
            else {
                log!("Error base64 decoding preset");
            }
        }
    }*/
    
}
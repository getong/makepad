use {
    std::sync::{Arc, Mutex},
    std::sync::mpsc,
    crate::{
        makepad_live_id::{live_id, LiveId},
        midi::*,
        cx::Cx,
        cx_api::CxOsApi,
        windows_crate::{
            Foundation::{
                EventRegistrationToken,
                TypedEventHandler,
            },
            Storage::Streams::{
                DataWriter,
                DataReader
            },
            Devices::Enumeration::{
                DeviceWatcher,
                DeviceInformation,
                DeviceInformationUpdate
            },
            Devices::Midi::{MidiInPort, MidiOutPort, IMidiOutPort, MidiMessageReceivedEventArgs},
        }
    },
};


type WindowsResult<T> = crate::windows_crate::core::Result<T>;

#[derive(Clone)]
pub struct OsMidiOutput(pub (crate) Arc<Mutex<WinRTMidiAccess >>);

impl OsMidiOutput {
    pub fn send(&self, port_id: Option<MidiPortId>, d: MidiData) {
        let _ =  self.0.lock().unwrap().event_sender.send(WinRTMidiEvent::SendMidi(port_id, d));
    }
}

type InputSenders = Arc<Mutex<Vec<mpsc::Sender<(MidiPortId, MidiData) >> >>;

#[derive(Clone)]
pub struct WinRTMidiPort {
    winrt_id: String,
    desc: MidiPortDesc
}

#[derive(Clone)]
pub struct WinRTMidiInput {
    port_id: MidiPortId,
    event_token: EventRegistrationToken,
    midi_input: MidiInPort,
}

#[derive(Clone)]
pub struct WinRTMidiOutput {
    port_id: MidiPortId,
    midi_output: IMidiOutPort,
}

pub struct WinRTMidiAccess {
    input_senders: InputSenders,
    event_sender: mpsc::Sender<WinRTMidiEvent>,
    descs: Vec<MidiPortDesc>,
}

#[derive(Clone)]
enum WinRTMidiEvent {
    UpdateDevices,
    SendMidi(Option<MidiPortId>, MidiData),
    Terminate,
    UseMidiInputs(Vec<MidiPortId>),
    UseMidiOutputs(Vec<MidiPortId>),
}

impl WinRTMidiAccess {
    
    async fn create_midi_in_port(winrt_id: &str) -> WindowsResult<MidiInPort> {
        let port = MidiInPort::FromIdAsync(&winrt_id.into()) ? .await ?;
        Ok(port)
    }
    
    async fn create_midi_out_port(winrt_id: &str) -> WindowsResult<IMidiOutPort> {
        let port = MidiOutPort::FromIdAsync(&winrt_id.into()) ? .await ?;
        Ok(port)
    }
    
    
    async fn get_ports_list() -> WindowsResult<Vec<WinRTMidiPort >> {
        let input_query = MidiInPort::GetDeviceSelector().unwrap();
        let mut ports = Vec::new();
        let collection = DeviceInformation::FindAllAsyncAqsFilter(&input_query) ? .await ?;
        for item in collection {
            let winrt_id = item.Id().unwrap().to_string();
            ports.push(WinRTMidiPort {
                desc: MidiPortDesc {
                    name: item.Name().unwrap().to_string(),
                    port_id: LiveId::from_str_unchecked(&winrt_id).into(),
                    port_type: MidiPortType::Input,
                },
                winrt_id
            });
        }
        let output_query = MidiOutPort::GetDeviceSelector().unwrap();
        let collection = DeviceInformation::FindAllAsyncAqsFilter(&output_query) ? .await ?;
        for item in collection {
            let winrt_id = item.Id().unwrap().to_string();
            ports.push(WinRTMidiPort {
                desc: MidiPortDesc {
                    name: item.Name().unwrap().to_string(),
                    port_id: LiveId::from_str_unchecked(&winrt_id).into(),
                    port_type: MidiPortType::Output,
                },
                winrt_id
            });
        }
        Ok(ports)
    }
    
    pub fn new() -> Arc<Mutex<Self >> {
        
        let (watch_sender, watch_receiver) = mpsc::channel();
        let input_senders = InputSenders::default();
        let midi_access = Arc::new(Mutex::new(Self {
            descs: Vec::new(),
            event_sender: watch_sender.clone(),
            input_senders,
        }));
        let midi_access_clone = midi_access.clone();
        
        std::thread::spawn(move || {
            
            let mut ports_list = Vec::new();
            
            let mut midi_inputs = Vec::new();
            let mut midi_outputs = Vec::new();
            
            // initiate device list update
            watch_sender.send(WinRTMidiEvent::UpdateDevices).unwrap();
            // now lets watch device changes
            let query = MidiInPort::GetDeviceSelector().unwrap();
            let input_watcher = DeviceInformation::CreateWatcherAqsFilter(&query).unwrap();
            let query = MidiInPort::GetDeviceSelector().unwrap();
            let output_watcher = DeviceInformation::CreateWatcherAqsFilter(&query).unwrap();
            
            fn bind_watcher(watch_sender: mpsc::Sender::<WinRTMidiEvent>, watcher: &DeviceWatcher) {
                let sender = watch_sender.clone();
                watcher.Added(&TypedEventHandler::<DeviceWatcher, DeviceInformation>::new(move | _, _ | {
                    let _ = sender.send(WinRTMidiEvent::UpdateDevices);
                    Ok(())
                })).unwrap();
                let sender = watch_sender.clone();
                watcher.Removed(&TypedEventHandler::<DeviceWatcher, DeviceInformationUpdate>::new(move | _, _ | {
                    let _ = sender.send(WinRTMidiEvent::UpdateDevices);
                    Ok(())
                })).unwrap();
                let sender = watch_sender.clone();
                watcher.Updated(&TypedEventHandler::<DeviceWatcher, DeviceInformationUpdate>::new(move | _, _ | {
                    let _ = sender.send(WinRTMidiEvent::UpdateDevices);
                    Ok(())
                })).unwrap();
                let sender = watch_sender.clone();
                watcher.EnumerationCompleted(&TypedEventHandler::new(move | _, _ | {
                    let _ = sender.send(WinRTMidiEvent::UpdateDevices);
                    Ok(())
                })).unwrap();
            }
            
            bind_watcher(watch_sender.clone(), &input_watcher);
            bind_watcher(watch_sender.clone(), &output_watcher);
            input_watcher.Start().unwrap();
            output_watcher.Start().unwrap();
            
            while let Ok(msg) = watch_receiver.recv() {
                match msg {
                    WinRTMidiEvent::UpdateDevices => {
                        ports_list = futures::executor::block_on(Self::get_ports_list()).unwrap();
                        let mut descs = Vec::new();
                        for port in &ports_list {
                            descs.push(port.desc.clone());
                        }
                        midi_access_clone.lock().unwrap().descs = descs;
                        Cx::post_signal(live_id!(WinRTMidiPortsChanged).into());
                    }
                    WinRTMidiEvent::Terminate => {
                        break;
                    }
                    WinRTMidiEvent::UseMidiOutputs(ports) => {
                        //let cself = midi_access_clone.lock().unwrap();
                        // find all ports we want enabled
                        for port_id in &ports {
                            if let Some(port) = ports_list.iter_mut().find( | p | p.desc.port_id == *port_id && p.desc.port_type.is_output()) {
                                // open this output
                                let midi_output = futures::executor::block_on(Self::create_midi_out_port(&port.winrt_id)).unwrap();
                                midi_outputs.push(WinRTMidiOutput{
                                    port_id: *port_id,
                                    midi_output
                                });
                            }
                        }
                        // and the ones disabled
                        for port in &mut ports_list {
                            if ports.iter().find( | p | **p == port.desc.port_id).is_none() && port.desc.port_type.is_output() {
                                // close this output
                                if let Some(index) = midi_outputs.iter().position(|v| v.port_id == port.desc.port_id){
                                    let out = &midi_outputs[index];
                                    out.midi_output.Close().unwrap();
                                    midi_outputs.remove(index);
                                }
                            }
                        }
                    }
                    WinRTMidiEvent::UseMidiInputs(ports) => {
                        // find all ports we want enabled
                        for port_id in &ports {
                            if let Some(port) = ports_list.iter_mut().find( | p | p.desc.port_id == *port_id && p.desc.port_type.is_input()) {
                                // open this input
                                let midi_input = futures::executor::block_on(Self::create_midi_in_port(&port.winrt_id)).unwrap();
                                
                                let input_senders = midi_access_clone.lock().unwrap().input_senders.clone();
                                let port_id = *port_id;
                                let event_token = midi_input.MessageReceived(&TypedEventHandler::<MidiInPort, MidiMessageReceivedEventArgs>::new(move | _, msg | {
                                    let raw_data = msg.as_ref().unwrap().Message().unwrap().RawData().unwrap();
                                    let data_reader = DataReader::FromBuffer(&raw_data).unwrap();
                                    let mut data = [0u8;3];
                                    if data_reader.ReadBytes(&mut data).is_ok(){
                                        let mut senders = input_senders.lock().unwrap();
                                        senders.retain( | s | {
                                            s.send((port_id, MidiData {
                                                data,
                                            })).is_ok()
                                        });
                                    }
                                    Ok(())
                                })).unwrap();
                                midi_inputs.push(WinRTMidiInput{
                                    event_token,
                                    port_id,
                                    midi_input
                                });
                            }
                        }
                        // and the ones disabled
                        for port in &mut ports_list {
                            if ports.iter().find( | p | **p == port.desc.port_id).is_none() && port.desc.port_type.is_input() {
                                //close this input
                                if let Some(index) = midi_inputs.iter().position(|v| v.port_id == port.desc.port_id){
                                    let inp = &midi_inputs[index];
                                    inp.midi_input.RemoveMessageReceived(inp.event_token).unwrap();
                                    inp.midi_input.Close().unwrap();
                                    midi_inputs.remove(index);
                                }
                            }
                        }
                    }
                    WinRTMidiEvent::SendMidi(port_id, midi_data)=>{
                        let writer = DataWriter::new().unwrap();
                        writer.WriteBytes(&midi_data.data).unwrap();
                        let buffer = writer.DetachBuffer().unwrap();
                        for output in &mut midi_outputs {
                            if port_id.is_none() || output.port_id == port_id.unwrap() {
                                output.midi_output.SendBuffer(&buffer).unwrap();
                            }
                        }
                    }                    
                }
            }
            input_watcher.Stop().unwrap();
            output_watcher.Stop().unwrap();
        });
        
        
        //output_watcher.Start().unwrap();
        // alrighty lets initialize midi.
        Cx::post_signal(live_id!(WinRTMidiInputsChanged).into());
        midi_access
    }
    
    pub fn create_midi_input(&self) -> MidiInput {
        let senders = self.input_senders.clone();
        let (send, recv) = mpsc::channel();
        senders.lock().unwrap().push(send);
        MidiInput(Some(recv))
    }
    
    pub fn use_midi_outputs(&mut self, ports: &[MidiPortId]) {
        self.event_sender.send(WinRTMidiEvent::UseMidiOutputs(ports.to_vec())).unwrap();
    }
    
    pub fn use_midi_inputs(&mut self, ports: &[MidiPortId]) {
        self.event_sender.send(WinRTMidiEvent::UseMidiInputs(ports.to_vec())).unwrap();
    }
    
    pub fn get_descs(&self) -> Vec<MidiPortDesc> {
        self.descs.clone()
    }
    
}
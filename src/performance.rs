//! Opt-in native performance harness. Synthetic SQLite data only.
use gpui::*;
use serde_json::{json,Value};
use std::time::Instant;
pub fn stats(mut samples:Vec<f64>)->Value{
    samples.sort_by(f64::total_cmp);let n=samples.len();
    let p=|q:f64|samples[((n as f64*q).ceil()as usize).saturating_sub(1).min(n-1)];
    json!({"samples":n,"median_ms":p(0.5),"p95_ms":p(0.95),"max_ms":samples[n-1],"mean_ms":samples.iter().sum::<f64>()/n as f64,"over_16_67_ms":samples.iter().filter(|&&v|v>1000./60.).count(),"over_33_33_ms":samples.iter().filter(|&&v|v>1000./30.).count()})
}
pub fn ms(start:Instant)->f64{start.elapsed().as_secs_f64()*1000.}
pub fn fixtures()->Vec<(String,String)>{
    let paragraph="A quick note with **formatted text**, café, ideas 🌱 and a [reference](https://example.com). The editor should stay responsive as this document grows. Work locally, type naturally, and keep the next step in view.";
    let mut fixtures=Vec::new();
    for n in [20,200,2000,8000]{let source=(0..n).map(|i|if i%15==0{format!("## Section {i}")}else{format!("Paragraph {i}: {paragraph}")}).collect::<Vec<_>>().join("\n\n");fixtures.push((format!("blocks-{n}"),source));}
    let table="| Name | Value | Status |\n| :--- | ---: | --- |\n".to_owned()+(0..400).map(|i|format!("| **Item {i}** | {i} | Ready |\n")).collect::<String>().as_str();fixtures.push(("table-400-rows".into(),table));
    fixtures.push(("code-5000-lines".into(),format!("```tsx\n{}\n```",(0..5000).map(|i|format!("const Component{i} = () => <div>Hello {i}</div>;" )).collect::<Vec<_>>().join("\n"))));
    fixtures.push(("paragraph-512kb".into(),"Words for a long paragraph without block boundaries. ".repeat(10_000)));
    fixtures
}
#[link(name="QuartzCore",kind="framework")]
extern "C" {fn CACurrentMediaTime()->f64;}
pub fn media_time()->f64{unsafe{CACurrentMediaTime()}}
pub fn presented_stats(trace:&str,start:f64,end:f64)->Value{
    let mut times=std::fs::read_to_string(trace).unwrap_or_default().lines().filter_map(|line|serde_json::from_str::<Value>(line).ok()?.get("presented_time_s")?.as_f64()).filter(|&t|t>=start&&t<=end).collect::<Vec<_>>();
    times.sort_by(f64::total_cmp);times.dedup();
    let intervals=times.windows(2).map(|p|(p[1]-p[0])*1000.).collect::<Vec<_>>();
    if intervals.len()<10{return json!({"valid":false,"presented_frames":times.len(),"reason":"Insufficient drawable presentation timestamps"});}
    let mean=intervals.iter().sum::<f64>()/intervals.len()as f64;
    json!({"valid":true,"presented_frames":times.len(),"effective_fps":1000./mean,"frame_intervals":stats(intervals),"source":"MTLDrawable.presentedTime / addPresentedHandler"})
}
pub fn run(output:String){
    let trace=format!("{output}.frames.jsonl");std::fs::write(&trace,"").unwrap();std::env::set_var("SPARKPAD_FRAME_TRACE",&trace);
    let start=Instant::now();let app=Application::new().with_assets(crate::assets::Assets);crate::assets::register_fonts(&app.text_system());
    app.run(move|cx|{
        // Executables launched outside an .app bundle default to a prohibited
        // activation policy. Match the native app so AppKit drives real frames.
        use objc::{class,msg_send,sel,sel_impl};
        unsafe{let native:cocoa::base::id=msg_send![class!(NSApplication),sharedApplication];let _:()=msg_send![native,setActivationPolicy:0_i64];}
        gpui_component::init(cx);crate::block_editor::init(cx);crate::ui::benchmark_native(output,start,cx);});
}

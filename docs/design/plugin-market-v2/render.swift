// Review artwork only. Not application UI code.
import AppKit
let root = URL(fileURLWithPath: CommandLine.arguments[1])
let W: CGFloat = 1120, H: CGFloat = 760
func color(_ hex: UInt32) -> NSColor { NSColor(red: CGFloat((hex >> 16) & 255)/255, green: CGFloat((hex >> 8) & 255)/255, blue: CGFloat(hex & 255)/255, alpha: 1) }
let ink: UInt32 = 0x1f2329, muted: UInt32 = 0x7d8590, blue: UInt32 = 0x1f6fe5, line: UInt32 = 0xe4e6ea
func box(_ x: CGFloat,_ y: CGFloat,_ w: CGFloat,_ h: CGFloat,_ fill: UInt32 = 0xffffff,_ r: CGFloat = 7,_ stroke: Bool = false) {
 let path = NSBezierPath(roundedRect: NSRect(x:x,y:H-y-h,width:w,height:h),xRadius:r,yRadius:r); color(fill).setFill();path.fill()
 if stroke { color(line).setStroke();path.lineWidth=0.8;path.stroke() }
}
func text(_ s: String,_ x: CGFloat,_ y: CGFloat,_ size: CGFloat = 13,_ c: UInt32 = 0x1f2329,_ bold: Bool = false) {
 let font = NSFont.systemFont(ofSize:size,weight:bold ? .semibold:.regular)
 (s as NSString).draw(at:NSPoint(x:x,y:H-y-size-4),withAttributes:[.font:font,.foregroundColor:color(c)])
}
func icon(_ s:String,_ x:CGFloat,_ y:CGFloat,_ size:CGFloat = 17,_ c:UInt32 = 0x5c636d) {
 if let image=NSImage(systemSymbolName:s,accessibilityDescription:nil) {
 let tinted=NSImage(size:NSSize(width:size,height:size));tinted.lockFocus();image.draw(in:NSRect(x:0,y:0,width:size,height:size));color(c).setFill();NSRect(x:0,y:0,width:size,height:size).fill(using:.sourceAtop);tinted.unlockFocus();tinted.draw(in:NSRect(x:x,y:H-y-size,width:size,height:size))
 }
}
func button(_ s:String,_ x:CGFloat,_ y:CGFloat,_ width:CGFloat = 85,_ primary:Bool = false){box(x,y,width,30,primary ? blue:0xffffff,6,!primary);text(s,x+12,y+5,12,primary ? 0xffffff:ink)}
func divider(_ y:CGFloat,_ x:CGFloat = 228,_ w:CGFloat = 860){box(x,y,w,1,line,0)}
func base(_ active:String,_ title:String) {
 box(0,0,W,H,0xffffff,12);box(0,28,196,H-28,0xf1f2f4,0);box(195,28,1,H-28,line,0)
 for (i,c) in [UInt32(0xff6059),0xffbd2e,0x28c840].enumerated(){box(CGFloat(13+i*19),9,11,11,c,6)}
 let entries:[(String,String,CGFloat)] = [("主页","house",88),("Agent","cpu",126),("社区","bubble.left.and.bubble.right",164),("课程表","calendar",244),("校园导航","safari",282),("成绩查询","chart.bar.doc.horizontal",320),("一键评教","checklist",358),("插件市场","square.grid.2x2",626),("应用设置","gearshape",664)]
 text("插件",16,211,11,muted)
 for (name,symbol,y) in entries { if active == name {box(8,y-5,180,32,0xe1e8f3)};icon(symbol,20,y+2,17,active==name ? blue:0x5c636d);text(name,48,y,13,active==name ? blue:ink) }
 box(185,380,21,35,0xf7f8fa,5,true);icon("chevron.left",190,392,10)
 box(10,710,176,40,0xe9ebef);icon("person.crop.circle",21,718,24);text("校园账号",55,714,12,ink,true);text("当前账号",55,731,10,muted)
 text(title,228,59,22,ink,true)
}
func render(_ name:String,_ body:()->Void){let image=NSImage(size:NSSize(width:W,height:H));image.lockFocus();body();image.unlockFocus();let rep=NSBitmapImageRep(data:image.tiffRepresentation!)!;try! rep.representation(using:.png,properties:[:])!.write(to:root.appendingPathComponent(name+".png"))}
func tabs(_ selected:Int){
 box(228,112,354,32,0xf1f2f4);box(CGFloat(230+selected*117),114,116,28,0xffffff,5)
 for (i,name) in ["校园功能","远端市场","已安装"].enumerated(){text(name,CGFloat(257+i*117),119,12,i==selected ? ink:muted,i==selected)}
}
func toggle(_ x:CGFloat,_ y:CGFloat,_ on:Bool,_ disabled:Bool=false){
 box(x,y,33,19,disabled ? 0xe5e7eb:(on ? blue:0xc9cdd3),10);box(x+(on ? 16:2),y+2,15,15,0xffffff,8)
}
render("01-campus") {
 base("插件市场","插件市场");tabs(0)
 box(840,59,248,30,0xf7f8fa,6,true);icon("magnifyingglass",850,67,14);text("搜索校园功能",874,65,12,muted)
 text("学习与教务",228,179,12,muted)
 let items=[("calendar","课程表","课程安排与考试时间","已添加"),("checklist","一键评教","批量填写、预览与提交","添加"),("chart.bar.doc.horizontal","成绩查询","成绩记录与自选课程统计","添加"),("person.badge.clock","考勤查询","课程考勤与签到记录","添加"),("building.2","空闲教室","按校区、日期和节次查询","添加")]
 for (i,row) in items.enumerated(){let y=CGFloat(210+i*63);icon(row.0,245,y+10,23,blue);text(row.1,285,y,14,ink,true);text(row.2,285,y+23,12,muted);button(row.3,1006,y+8,70);divider(y+54)}
 text("校园生活",228,548,12,muted)
 for (i,row) in [("creditcard","校园卡","余额与消费记录"),("books.vertical","图书馆","座位查询与预约")].enumerated(){let y=CGFloat(577+i*65);icon(row.0,245,y+10,23,blue);text(row.1,285,y,14,ink,true);text(row.2,285,y+23,12,muted);button("添加",1006,y+8,70);divider(y+54)}
}
render("02-remote") {
 base("插件市场","插件市场");tabs(1)
 box(840,59,248,30,0xf7f8fa,6,true);icon("magnifyingglass",850,67,14);text("搜索远端插件",874,65,12,muted)
 text("全部",228,179,12,blue,true);text("学习",290,179,12,muted);text("效率",352,179,12,muted);text("校园生活",414,179,12,muted)
 let items=[("bell","课程提醒","课前提醒与上课地点速览","下载"),("timer","自习计时","专注计时与学习记录","下载"),("bookmark","网页收藏","管理常用链接与分类","下载"),("doc.text","课程资料","按课程整理本地资料","打开")]
 for (i,row) in items.enumerated(){let y=CGFloat(222+i*94);box(245,y+5,44,44,0xeaf1fd,9);icon(row.0,256,y+16,22,blue);text(row.1,307,y,15,ink,true);text(row.2,307,y+26,12,muted);text("XJTU Launcher",307,y+48,11,muted);button(row.3,1002,y+14,74);divider(y+80)}
}
render("03-installed") {
 base("插件市场","插件市场");tabs(2);button("导入本地插件",951,58,137)
 text("插件",245,182,12,muted);text("启用",791,182,12,muted);text("侧栏显示",870,182,12,muted)
 let items=[("calendar","课程表","校园功能",true,true),("checklist","一键评教","校园功能",true,true),("bell","课程提醒","远端插件",true,false),("doc.text","课程资料","远端插件",false,false)]
 for (i,row) in items.enumerated(){let y=CGFloat(226+i*89);icon(row.0,245,y+12,23,row.3 ? blue:muted);text(row.1,286,y,14,ink,true);text(row.2,286,y+26,12,muted);toggle(790,y+14,row.3);toggle(882,y+14,row.4,!row.3);icon("ellipsis",1043,y+15,20);divider(y+72)}
}
func sourceSettings(){
 base("应用设置","设置");text("外观与交互",228,113,12,muted);text("模型服务",336,113,12,muted);text("插件服务",432,113,12,blue,true);text("软件更新",528,113,12,muted);divider(148)
 text("插件服务端",228,190,15,ink,true)
 text("服务端网址",228,243,13,ink,true);icon("lock",683,247,13);text("https://plugins.example.com",709,243,13,muted);button("确认更改",979,237,109)
 divider(294);text("恢复默认地址",228,326,13);button("恢复默认",979,320,109)
 text("插件更新",228,413,15,ink,true);text("自动检查插件更新",228,465,13);toggle(1041,467,true);divider(509)
}
render("04-source-locked"){sourceSettings()}
render("05-source-warning") {
 sourceSettings();NSColor.black.withAlphaComponent(0.18).setFill();NSRect(x:0,y:0,width:W,height:H).fill()
 box(355,238,410,267,0xffffff,13,true);icon("exclamationmark.triangle.fill",535,264,43,0xd68b14);text("更改插件服务端？",465,324,18,ink,true)
 text("更改后，插件列表、下载和更新将来自新地址。",390,368,13);text("不可信的服务端可能提供有害插件。",414,393,13,muted)
 button("取消",380,447,177);button("了解风险，继续",570,447,170,true)
}
render("06-source-editing") {
 base("应用设置","设置");text("外观与交互",228,113,12,muted);text("模型服务",336,113,12,muted);text("插件服务",432,113,12,blue,true);text("软件更新",528,113,12,muted);divider(148)
 text("插件服务端",228,190,15,ink,true);text("服务端网址",228,246,13,ink,true)
 box(500,239,386,33,0xffffff,6,true);text("https://plugins.example.com",512,246,13);button("取消",903,240,77);button("保存",991,240,97,true);divider(294)
}
render("07-updates") {
 base("应用设置","设置");text("外观与交互",228,113,12,muted);text("模型服务",336,113,12,muted);text("插件服务",432,113,12,muted);text("软件更新",528,113,12,blue,true);divider(148)
 icon("arrow.down.app",240,197,38,blue);text("XJTU Launcher",298,189,18,ink,true);text("当前版本 0.1.0",298,219,12,muted);button("检查更新",979,201,109)
 divider(280);text("自动检查更新",228,314,13);toggle(1041,316,true);divider(363)
 text("发现新版本",228,405,15,ink,true);text("0.1.1",228,440,13,muted)
 text("更新内容",228,494,13,ink,true);text("改善插件管理体验，修复已知问题。",228,526,13,muted)
 button("下载并更新",951,629,137,true)
}

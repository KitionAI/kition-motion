/**
 * Tessera icons for the Motion engine's commands that the SDK editor's rules cannot read (they draw
 * the neutral command mark otherwise): its own vocabulary, by command id. Registered once, when the
 * editor loads (`main.tsx`); ribbons and menus built from engine commands draw them.
 */
import { registerCommandIcons } from '@kition/sdk/editor'
import { ActivityIcon, BrushIcon, CameraIcon, CircleAlertIcon, ClapperboardIcon, ClipboardListIcon, ClockIcon, CpuIcon, DatabaseIcon, DownloadIcon, FileCodeIcon, GraduationCapIcon, HexagonIcon, HourglassIcon, InfoIcon, KeyboardIcon, LayoutTemplateIcon, LightbulbIcon, MessageSquareIcon, MousePointerClickIcon, ScanFaceIcon, SearchIcon, ShapesIcon, SlidersHorizontalIcon, SnowflakeIcon, SplineIcon, SunIcon, TagIcon, TerminalIcon, WaypointsIcon } from '@kition/tessera/icons/react'

registerCommandIcons({
  "app.capabilities": InfoIcon,
  "app.gpuInfo": CpuIcon,
  "app.keyboardShortcuts": KeyboardIcon,
  "camera.fromModel": CameraIcon,
  "camera.stereoRig": CameraIcon,
  "command.describe": InfoIcon,
  "edit.history": ClockIcon,
  "expr.errors": CircleAlertIcon,
  "face.model.download": DownloadIcon,
  "face.model.install": DownloadIcon,
  "face.models": ScanFaceIcon,
  "help.inAppTutorials": GraduationCapIcon,
  "help.reportIssue": MessageSquareIcon,
  "help.systemInfo": InfoIcon,
  "help.systemReport": ClipboardListIcon,
  "item.metadata": TagIcon,
  "job.wait": HourglassIcon,
  "layer.newPrimitive": ShapesIcon,
  "light.fromModel": LightbulbIcon,
  "mask.setClosed": HexagonIcon,
  "mask.setVertex": SplineIcon,
  "puppet.follow": WaypointsIcon,
  "render.preRender": ClapperboardIcon,
  "roto.freeze": SnowflakeIcon,
  "roto.span": BrushIcon,
  "roto.unfreeze": SunIcon,
  "scope.analyze": ActivityIcon,
  "script.run": FileCodeIcon,
  "scriptWindow.click": MousePointerClickIcon,
  "scriptWindow.set": SlidersHorizontalIcon,
  "storage.persist": DatabaseIcon,
  "view.assign3dShortcut": KeyboardIcon,
  "warp.analyze": SearchIcon,
  "window.scriptPanel": TerminalIcon,
  "window.workspace": LayoutTemplateIcon,
})

local Blitbuffer = require("ffi/blitbuffer")
local BottomContainer = require("ui/widget/container/bottomcontainer")
local Button = require("ui/widget/button")
local ButtonTable = require("ui/widget/buttontable")
local CenterContainer = require("ui/widget/container/centercontainer")
local Device = require("device")
local Font = require("ui/font")
local FrameContainer = require("ui/widget/container/framecontainer")
local Geom = require("ui/geometry")
local HorizontalGroup = require("ui/widget/horizontalgroup")
local HorizontalSpan = require("ui/widget/horizontalspan")
local InfoMessage = require("ui/widget/infomessage")
local InputDialog = require("ui/widget/inputdialog")
local LeftContainer = require("ui/widget/container/leftcontainer")
local LineWidget = require("ui/widget/linewidget")
local Size = require("ui/size")
local TextWidget = require("ui/widget/textwidget")
local TitleBar = require("ui/widget/titlebar")
local UIManager = require("ui/uimanager")
local VerticalGroup = require("ui/widget/verticalgroup")
local VerticalSpan = require("ui/widget/verticalspan")
local _ = require("gettext+")
local Screen = Device.screen

local Backend = require("Backend")
local ErrorDialog = require("ErrorDialog")
local LoadingDialog = require("LoadingDialog")
local TrackerSearchDialog = require("TrackerSearchDialog")

--- @class TrackingMenu
--- @field source_id string
--- @field manga_id string
--- @field manga_title string
--- @field on_close function
local TrackingMenu = {}
local TrackingMenu_mt = { __index = TrackingMenu }

function TrackingMenu:new(o)
  o = o or {}
  setmetatable(o, TrackingMenu_mt)
  return o
end

function TrackingMenu:show()
  self:_loadAndDisplay()
end

function TrackingMenu:_loadAndDisplay()
  local services_resp = Backend.getTrackerServices()
  if services_resp.type == "ERROR" then
    ErrorDialog:new{message = services_resp.message}:show()
    return
  end

  local services = services_resp.body
  local logged_in = {}
  for _, svc in ipairs(services) do
    if svc.logged_in then
      table.insert(logged_in, svc.tracker)
    end
  end

  if #logged_in == 0 then
    UIManager:show(InfoMessage:new{
      text = _("No trackers are logged in. Go to Settings > Tracking to log in."),
    })
    return
  end

  -- Build the tracking section for each logged-in tracker.
  local items = {}
  for _, tracker in ipairs(logged_in) do
    local entries_resp = Backend.getTrackerEntries(tracker)
    local entries = {}
    if entries_resp.type == "SUCCESS" then
      for _, entry in ipairs(entries_resp.body) do
        if entry.manga_source_id == self.source_id and entry.manga_id == self.manga_id then
          table.insert(entries, entry)
        end
      end
    end

    local tracker_name = tracker == "anilist" and "AniList" or "MyAnimeList"

    if #entries > 0 then
      local entry = entries[1]
      local progress_text = tostring(entry.last_chapter_read)
      if entry.total_chapters and entry.total_chapters > 0 then
        progress_text = progress_text .. " / " .. tostring(entry.total_chapters)
      end
      local status_text = entry.status or ""

      table.insert(items, {
        tracker = tracker,
        tracker_name = tracker_name,
        linked = true,
        progress_text = progress_text,
        status_text = status_text,
        score = entry.score,
      })
    else
      table.insert(items, {
        tracker = tracker,
        tracker_name = tracker_name,
        linked = false,
      })
    end
  end

  -- Build the widget.
  local width = math.floor(Screen:getWidth() * 0.9)
  local content_widgets = {}

  table.insert(content_widgets, VerticalSpan:new{ width = Size.padding.large })

  for _, item in ipairs(items) do
    local row = HorizontalGroup:new{}

    table.insert(row, TextWidget:new{
      text = item.tracker_name,
      face = Font:getFace("ffont"),
      bold = true,
    })
    table.insert(row, HorizontalSpan:new{ width = Size.padding.large })

    if item.linked then
      local progress = item.progress_text .. " ch"
      if item.status_text ~= "" then
        progress = progress .. " (" .. item.status_text .. ")"
      end
      table.insert(row, TextWidget:new{
        text = progress,
        face = Font:getFace("smallffont"),
      })
      table.insert(row, HorizontalSpan:new{ width = Size.padding.large })
      table.insert(row, Button:new{
        text = _("Unlink"),
        callback = function()
          self:_unlinkTracker(item.tracker)
        end,
      })
    else
      table.insert(row, TextWidget:new{
        text = _("Not linked"),
        face = Font:getFace("smallffont"),
        foreground = Blitbuffer.COLOR_DARK_GRAY,
      })
      table.insert(row, HorizontalSpan:new{ width = Size.padding.large })
      table.insert(row, Button:new{
        text = _("Link"),
        callback = function()
          self:_linkTracker(item.tracker)
        end,
      })
    end

    table.insert(content_widgets, LeftContainer:new{
      dimen = Geom:new{ w = width, h = Size.item.height_default },
      row,
    })
    table.insert(content_widgets, VerticalSpan:new{ width = Size.padding.default })
  end

  -- Sync queue badge.
  local queue_resp = Backend.getSyncQueue()
  if queue_resp.type == "SUCCESS" and #queue_resp.body > 0 then
    table.insert(content_widgets, VerticalSpan:new{ width = Size.padding.default })
    table.insert(content_widgets, LineWidget:new{
      dimen = Geom:new{ w = width, h = Size.line.thin },
      background = Blitbuffer.COLOR_LIGHT_GRAY,
    })
    table.insert(content_widgets, VerticalSpan:new{ width = Size.padding.default })
    table.insert(content_widgets, TextWidget:new{
      text = T(_("%1 updates pending sync"), #queue_resp.body),
      face = Font:getFace("smallffont"),
      foreground = Blitbuffer.COLOR_DARK_GRAY,
    })
  end

  -- Pull sync button.
  table.insert(content_widgets, VerticalSpan:new{ width = Size.padding.large })
  table.insert(content_widgets, Button:new{
    text = _("Sync with trackers"),
    callback = function()
      self:_pullSync()
    end,
  })

  local frame = FrameContainer:new{
    width = width,
    background = Blitbuffer.COLOR_WHITE,
    padding = Size.padding.large,
    padding_bottom = Size.padding.large,
    VerticalGroup:new{
      align = "left",
      unpack(content_widgets),
    },
  }

  local title_bar = TitleBar:new{
    title = _("Tracking"),
    width = width,
    close_callback = function()
      UIManager:close(self.dialog)
      if self.on_close then
        self.on_close()
      end
    end,
  }

  self.dialog = VerticalGroup:new{
    align = "center",
    title_bar,
    frame,
  }

  UIManager:show(self.dialog)
end

function TrackingMenu:_linkTracker(tracker)
  local search_dialog
  search_dialog = TrackerSearchDialog:new{
    tracker = tracker,
    source_id = self.source_id,
    manga_id = self.manga_id,
    manga_title = self.manga_title,
    on_linked = function()
      UIManager:close(self.dialog)
      self:_loadAndDisplay()
    end,
  }
  search_dialog:show()
end

function TrackingMenu:_unlinkTracker(tracker)
  local confirm = require("ui/widget/confirmbox")
  UIManager:show(confirm:new{
    text = _("Unlink this manga from the tracker?"),
    ok_text = _("Unlink"),
    ok_callback = function()
      local resp = Backend.unlinkMangaFromTracker(tracker, self.source_id, self.manga_id)
      if resp.type == "ERROR" then
        ErrorDialog:new{message = resp.message}:show()
        return
      end
      UIManager:close(self.dialog)
      self:_loadAndDisplay()
    end,
  })
end

function TrackingMenu:_pullSync()
  local loading = LoadingDialog:simple(_("Syncing with trackers..."))

  local resp = Backend.pullTrackerSync()
  UIManager:close(loading)

  if resp.type == "ERROR" then
    ErrorDialog:new{message = resp.message}:show()
    return
  end

  local messages = resp.body
  if #messages == 0 then
    UIManager:show(InfoMessage:new{
      text = _("Everything is up to date."),
    })
  else
    local text = table.concat(messages, "\n")
    UIManager:show(InfoMessage:new{
      text = text,
    })
  end

  UIManager:close(self.dialog)
  self:_loadAndDisplay()
end

return TrackingMenu

local Blitbuffer = require("ffi/blitbuffer")
local Button = require("ui/widget/button")
local Device = require("device")
local Font = require("ui/font")
local FrameContainer = require("ui/widget/container/framecontainer")
local Geom = require("ui/geometry")
local HorizontalGroup = require("ui/widget/horizontalgroup")
local HorizontalSpan = require("ui/widget/horizontalspan")
local InfoMessage = require("ui/widget/infomessage")
local InputDialog = require("ui/widget/inputdialog")
local LeftContainer = require("ui/widget/container/leftcontainer")
local Size = require("ui/size")
local TextWidget = require("ui/widget/textwidget")
local TitleBar = require("ui/widget/titlebar")
local UIManager = require("ui/uimanager")
local VerticalGroup = require("ui/widget/verticalgroup")
local VerticalSpan = require("ui/widget/verticalspan")
local _ = require("gettext+")
local Screen = Device.screen
local T = require("ffi/util").template

local Backend = require("Backend")
local ErrorDialog = require("ErrorDialog")
local LoadingDialog = require("LoadingDialog")

--- Dialog that searches a tracker's catalog and lets the user pick a manga to link.
--- @class TrackerSearchDialog
--- @field tracker string
--- @field source_id string
--- @field manga_id string
--- @field manga_title string
--- @field on_linked function|nil
local TrackerSearchDialog = {}
local TrackerSearchDialog_mt = { __index = TrackerSearchDialog }

function TrackerSearchDialog:new(o)
  o = o or {}
  setmetatable(o, TrackerSearchDialog_mt)
  return o
end

function TrackerSearchDialog:show()
  local width = math.floor(Screen:getWidth() * 0.9)

  -- Step 1: Input dialog for search query.
  local input_dialog
  input_dialog = InputDialog:new{
    title = T(_("Search on %1"), self.tracker == "anilist" and "AniList" or "MyAnimeList"),
    input = self.manga_title or "",
    input_hint = _("Manga title"),
    buttons = {
      {
        {
          text = _("Cancel"),
          id = "close",
          callback = function()
            UIManager:close(input_dialog)
          end,
        },
        {
          text = _("Search"),
          is_enter_default = true,
          callback = function()
            local query = input_dialog:getInputText()
            if query == "" then
              return
            end
            UIManager:close(input_dialog)
            self:_doSearch(query, width)
          end,
        },
      },
    },
  }
  UIManager:show(input_dialog)
  input_dialog:onShowKeyboard()
end

function TrackerSearchDialog:_doSearch(query, width)
  local loading = LoadingDialog:simple(_("Searching..."))

  local resp = Backend.searchTrackerManga(self.tracker, query)
  UIManager:close(loading)

  if resp.type == "ERROR" then
    ErrorDialog:new{message = resp.message}:show()
    return
  end

  local results = resp.body
  if #results == 0 then
    UIManager:show(InfoMessage:new{
      text = _("No results found."),
    })
    return
  end

  self:_showResults(results, width)
end

function TrackerSearchDialog:_showResults(results, width)
  local content_widgets = {}

  table.insert(content_widgets, VerticalSpan:new{ width = Size.padding.large })

  for i, result in ipairs(results) do
    local row = HorizontalGroup:new{}

    -- Index number
    table.insert(row, TextWidget:new{
      text = tostring(i) .. ". ",
      face = Font:getFace("smallffont"),
      bold = true,
    })

    -- Title
    local title_text = result.title
    if result.total_chapters and result.total_chapters > 0 then
      title_text = title_text .. " (" .. tostring(result.total_chapters) .. " ch)"
    end
    table.insert(row, TextWidget:new{
      text = title_text,
      face = Font:getFace("smallffont"),
      max_width = width - Size.padding.large * 4,
    })

    table.insert(row, HorizontalSpan:new{ width = Size.padding.large })

    -- Link button
    table.insert(row, Button:new{
      text = _("Link"),
      callback = function()
        self:_linkResult(result)
      end,
    })

    table.insert(content_widgets, LeftContainer:new{
      dimen = Geom:new{ w = width, h = Size.item.height_default },
      row,
    })
    table.insert(content_widgets, VerticalSpan:new{ width = Size.padding.small })

    if i >= 20 then
      break
    end
  end

  local frame = FrameContainer:new{
    width = width,
    background = Blitbuffer.COLOR_WHITE,
    padding = Size.padding.large,
    VerticalGroup:new{
      align = "left",
      unpack(content_widgets),
    },
  }

  local title_bar = TitleBar:new{
    title = _("Search results"),
    width = width,
    close_callback = function()
      UIManager:close(self.results_dialog)
    end,
  }

  self.results_dialog = VerticalGroup:new{
    align = "center",
    title_bar,
    frame,
  }

  UIManager:show(self.results_dialog)
end

function TrackerSearchDialog:_linkResult(result)
  local loading = LoadingDialog:simple(_("Linking..."))

  local resp = Backend.linkMangaToTracker(
    self.tracker,
    self.source_id,
    self.manga_id,
    result.remote_id,
    result.title,
    result.total_chapters
  )
  UIManager:close(loading)

  if resp.type == "ERROR" then
    ErrorDialog:new{message = resp.message}:show()
    return
  end

  UIManager:close(self.results_dialog)

  local tracker_name = self.tracker == "anilist" and "AniList" or "MyAnimeList"
  UIManager:show(InfoMessage:new{
    text = T(_("Linked to %1"), tracker_name),
  })

  if self.on_linked then
    self.on_linked()
  end
end

return TrackerSearchDialog

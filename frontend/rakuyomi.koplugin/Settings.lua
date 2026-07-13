local Blitbuffer = require("ffi/blitbuffer")
local Button = require("ui/widget/button")
local FocusManager = require("ui/widget/focusmanager")
local FrameContainer = require("ui/widget/container/framecontainer")
local Geom = require("ui/geometry")
local HorizontalGroup = require("ui/widget/horizontalgroup")
local HorizontalSpan = require("ui/widget/horizontalspan")
local LeftContainer = require("ui/widget/container/leftcontainer")
local OverlapGroup = require("ui/widget/overlapgroup")
local Screen = require("device").screen
local Size = require("ui/size")
local TitleBar = require("ui/widget/titlebar")
local UIManager = require("ui/uimanager")
local VerticalGroup = require("ui/widget/verticalgroup")
local InfoMessage = require("ui/widget/infomessage")
local _ = require("gettext+")
local T = require("ffi/util").template
local Paths = require("Paths")
local Device = require("device")
local Font = require("ui/font")
local TextWidget = require("ui/widget/textwidget")
local ScrollableContainer = require("ui/widget/container/scrollablecontainer")
local MovableContainer = require("ui/widget/container/movablecontainer")

local Backend = require("Backend")
local ErrorDialog = require("ErrorDialog")
local SettingItem = require('widgets/SettingItem')

local ffi = require("ffi")

ffi.cdef [[
  struct sysinfo {
      long uptime;
      unsigned long loads[3];
      unsigned long totalram;
      unsigned long freeram;
      unsigned long sharedram;
      unsigned long bufferram;
      unsigned long totalswap;
      unsigned long freeswap;
      unsigned short procs;
      unsigned short pad;
      unsigned long totalhigh;
      unsigned long freehigh;
      unsigned int mem_unit;
      char _f[20-2*sizeof(long)-sizeof(int)];
  };
  int sysinfo(struct sysinfo *info);
]]

local function get_ram_via_ffi()
  local info = ffi.new("struct sysinfo")
  if ffi.C.sysinfo(info) == 0 then
    local mem_unit = info.mem_unit > 0 and info.mem_unit or 1
    local total_bytes = tonumber(info.totalram) * mem_unit
    local free_bytes = tonumber(info.freeram) * mem_unit

    return {
      total_mb = math.floor(total_bytes / 1024 / 1024),
      free_mb = math.floor(free_bytes / 1024 / 1024)
    }
  end
  return nil
end

-- REFACT This is duplicated from `SourceSettings` (pretty much all of it actually)
local Settings = FocusManager:extend {
  settings = {},
  tracker_services = nil,
  on_return_callback = nil,
  paths = { 0 }
}

local ram_info = get_ram_via_ffi()

--- @type [string, ValueDefinition][]
Settings.setting_value_definitions = {
  {
    nil,
    { type = 'divider', title = _("Library") }
  },
  {
    'library_view_mode',
    {
      type = 'enum',
      title = _("Library view mode"),
      options = {
        { label = _("Base"),  value = "base" },
        { label = _("Cover"), value = "cover" },
        { label = _("Grid"),  value = "grid" },
      },
      default = "cover",
    }
  },
  {
    'library_sorting_mode',
    {
      type = 'enum',
      title = _("Library sorting mode"),
      options = {
        { label = _("Order added ascending (Default)"),  value = 'ascending' },
        { label = _("Order added descending"),           value = 'descending' },
        { label = _("Title manga ascending"),            value = 'title_asc' },
        { label = _("Title manga descending"),           value = 'title_desc' },
        { label = _("Count unread chapters ascending"),  value = 'unread_asc' },
        { label = _("Count unread chapters descending"), value = 'unread_desc' },
        { label = _("Last read ascending"),              value = 'last_read_asc' },
        { label = _("Last read descending"),             value = 'last_read_desc' },
        { label = _("Source ascending"),                 value = 'source_asc' },
        { label = _("Source descending"),                value = 'source_desc' },
      }
    }
  },
  {
    'rakuyomi_items_per_page',
    {
      type = 'integer',
      title = _("Items per page (0 = auto)"),
      min_value = 0,
      max_value = 100,
      is_local = true,
      default = 0
    }
  },
  {
    'rakuyomi_grid_columns',
    {
      type = 'integer',
      title = _("Grid columns"),
      min_value = 2,
      max_value = 6,
      is_local = true,
      default = 3
    }
  },
  {
    'rakuyomi_grid_rows',
    {
      type = 'integer',
      title = _("Grid rows"),
      min_value = 0,
      max_value = 6,
      is_local = true,
      default = 0
    }
  },
  {
    'rakuyomi_grid_show_title',
    {
      type = 'boolean',
      title = _("Show title in grid mode"),
      default = true,
      is_local = true,
    }
  },
  {
    'rakuyomi_grid_show_metadata',
    {
      type = 'boolean',
      title = _("Show metadata in grid mode"),
      default = true,
      is_local = true,
    }
  },
  {
    'rakuyomi_tap_manga_action',
    {
      type = 'enum',
      title = _("Tap manga action"),
      options = {
        { label = _("Open chapter list"), value = "chapter_list" },
        { label = _("Continue reading"),  value = "continue_reading" },
      },
      default = "chapter_list",
      is_local = true,
    }
  },
  {
    'rakuyomi_skip_resume_confirm',
    {
      type = 'boolean',
      title = _("Skip resume reading confirmation"),
      default = false,
      is_local = true,
    }
  },
  {
    nil,
    { type = 'divider', title = _("Search") }
  },
  {
    'search_view_mode',
    {
      type = 'enum',
      title = _("Search view mode"),
      options = {
        { label = _("Base"),  value = "base" },
        { label = _("Cover"), value = "cover" },
        { label = _("Grid"),  value = "grid" },
      },
      default = "base",
    }
  },
  {
    nil,
    { type = 'divider', title = _("Reader") }
  },
  {
    'chapter_sorting_mode',
    {
      type = 'enum',
      title = _('Chapter sorting mode'),
      options = {
        { label = _("By chapter ascending"),  value = 'chapter_ascending' },
        { label = _("By chapter descending"), value = 'chapter_descending' },
      }
    }
  },
  {
    'preload_chapters',
    {
      type = 'integer',
      title = _("Preload chapters on reader open"),
      min_value = 0,
      max_value = 10,
      unit = 'chapters',
      default = 0
    }
  },
  {
    'optimize_image',
    {
      type = 'boolean',
      title = _("Optimize page images (experimental)"),
      default = false,
    }
  },
  {
    'concurrent_requests_pages',
    {
      type = 'integer',
      title = _("Concurrent page requests"),
      min_value = 1,
      max_value = 20,
      unit = 'pages',
      default = Device.isKindle() and 4 or 5
    }
  },
  {
    nil,
    { type = 'divider', title = _("Storage") }
  },
  {
    'storage_path',
    {
      type = 'path',
      title = _("Chapter storage path"),
      path_type = 'directory',
      default = Paths.getHomeDirectory() .. '/downloads',
    }
  },
  {
    'storage_size_limit_mb',
    {
      type = 'integer',
      title = _('Storage size limit'),
      min_value = 1,
      max_value = 10240,
      unit = 'MB'
    }
  },
  {
    'ram_storage_enabled',
    {
      type = 'boolean',
      title = _("Write chapters to RAM (protect eMMC)"),
      default = false,
    }
  },
  {
    'ram_storage_size_mb',
    {
      type = 'integer',
      title = _("RAM storage size. Your RAM is: " .. (ram_info and ram_info.total_mb or 0) .. " MB"),
      min_value = 8,
      max_value = ram_info and math.max(8, math.floor(ram_info.total_mb / 2)) or 32,
      unit = 'MB',
      default = 32,
    }
  },
  {
    nil,
    { type = 'divider', title = _("Sync & Updates") }
  },
  {
    'api_sync',
    {
      type = 'string',
      title = _("WebDAV Sync"),
      placeholder = 'user:password@example.com/folder',
    }
  },
  {
    'enabled_cron_check_mangas_update',
    {
      type = 'boolean',
      title = _("Enabled cron check for manga updates"),
      -- default = true,
    }
  },
  {
    'source_skip_cron',
    {
      type = 'string',
      title = _("Source IDs skip check update"),
      placeholder = 'com.manga,com.manga2'
    }
  },
  {
    nil,
    { type = 'divider', title = _("System") }
  },
  {
    'allow_commaneer_filemanager',
    {
      type = 'boolean',
      title = _("Allow requisition of the back button"),
      is_local = true,
      default = true
    }
  },
  {
    nil,
    { type = 'divider', title = _("Server") }
  },
  {
    'rakuyomi_auto_kill_server_delay',
    {
      type = 'enum',
      title = _("Auto-stop server when leaving library view"),
      options = {
        { label = _("Disabled"),         value = "disabled" },
        { label = _("Immediate"),        value = "immediate" },
        { label = _("After 30 seconds"), value = "30" },
        { label = _("After 1 minute"),   value = "60" },
        { label = _("After 5 minutes"),  value = "300" },
        { label = _("After 10 minutes"), value = "600" },
      },
      is_local = true,
      default = "disabled",
    }
  },
  {
    'rakuyomi_show_download_progress',
    {
      type = 'boolean',
      title = _("Show chapter download progress"),
      is_local = true,
      default = true
    }
  },
  {
    nil,
    { type = 'divider', title = _("Logging") }
  },
  {
    'rakuyomi_disable_logging',
    {
      type = 'boolean',
      title = _("Disable logging"),
      default = false,
      is_local = true,
    }
  },
}


--- @private
function Settings:init()
  self.dimen = Geom:new {
    x = 0,
    y = 0,
    w = self.width or Screen:getWidth(),
    h = self.height or Screen:getHeight(),
  }

  if self.dimen.w == Screen:getWidth() and self.dimen.h == Screen:getHeight() then
    self.covers_fullscreen = true -- hint for UIManager:_repaint()
  end

  local border_size = Size.border.window
  local padding = Size.padding.large

  self.inner_dimen = Geom:new {
    w = self.dimen.w - 2 * border_size,
    h = self.dimen.h - 2 * border_size,
  }

  self.item_width = self.inner_dimen.w - 2 * padding

  local vertical_group = VerticalGroup:new {
    align = "left",
  }

  for _, tuple in ipairs(Settings.setting_value_definitions) do
    local key = tuple[1]
    local definition = tuple[2]
    if definition.type == 'divider' then
      table.insert(vertical_group, TextWidget:new {
        text = definition.title,
        face = Font:getFace("cfont"),
        bold = true,
      })
    elseif definition.is_local then
      table.insert(vertical_group, SettingItem:new {
        show_parent = self,
        width = self.item_width,
        label = definition.title,
        value_definition = definition,
        value = G_reader_settings:readSetting(key, definition.default),
        on_value_changed_callback = function(new_value)
          G_reader_settings:saveSetting(key, new_value)
        end
      })
    else
      -- FIXME shouldn't the backend return the default value when unset?
      local value = self.settings[key]
      if key == 'storage_path' and value == nil then
        value = Paths.getHomeDirectory() .. '/downloads'
      end

      table.insert(vertical_group, SettingItem:new {
        show_parent = self,
        width = self.item_width,
        label = definition.title,
        value_definition = definition,
        value = value,
        on_value_changed_callback = function(new_value)
          return self:updateSetting(key, new_value)
        end
      })
    end
  end

  -- Tracking section: show each tracker service with login/logout.
  if self.tracker_services then
    table.insert(vertical_group, TextWidget:new {
      text = _("Tracking"),
      face = Font:getFace("cfont"),
      bold = true,
    })

    for _i, svc in ipairs(self.tracker_services) do
      local tracker_name = svc.tracker == "anilist" and "AniList" or "MyAnimeList"
      local status_text = svc.logged_in and _("Logged in") or _("Not logged in")

      local row = HorizontalGroup:new{}
      table.insert(row, TextWidget:new{
        text = tracker_name .. " — " .. status_text,
        face = Font:getFace("cfont", 18),
      })
      table.insert(row, HorizontalSpan:new{ width = Size.padding.large })

      if svc.logged_in then
        table.insert(row, Button:new{
          text = _("Log out"),
          callback = function()
            self:_logoutTracker(svc.tracker)
          end,
        })
      else
        table.insert(row, Button:new{
          text = _("Log in"),
          callback = function()
            self:_loginTracker(svc.tracker)
          end,
        })
      end

      table.insert(vertical_group, LeftContainer:new{
        dimen = Geom:new{ w = self.item_width, h = Size.item.height_default },
        row,
      })
    end
  end

  self.title_bar = TitleBar:new {
    title = _("Settings"),
    fullscreen = true,
    width = self.dimen.w,
    with_bottom_line = true,
    bottom_line_color = Blitbuffer.COLOR_DARK_GRAY,
    bottom_line_h_padding = padding,
    left_icon = "chevron.left",
    left_icon_tap_callback = function()
      self:onReturn()
    end,
    close_callback = function()
      self:onClose()
    end,
  }

  local scrollable = ScrollableContainer:new {
    dimen = Geom:new {
      w = self.dimen.w,
      h = self.dimen.h - self.title_bar.dimen.h,
    },
    vertical_group,
  }
  local content = OverlapGroup:new {
    allow_mirroring = false,
    dimen = self.inner_dimen:copy(),
    VerticalGroup:new {
      align = "left",
      self.title_bar,
      HorizontalGroup:new {
        HorizontalSpan:new { width = padding },
        scrollable
      }
    }
  }

  self[1] = FrameContainer:new {
    show_parent = self,
    width = self.dimen.w,
    height = self.dimen.h,
    padding = 0,
    margin = 0,
    bordersize = border_size,
    focusable = true,
    background = Blitbuffer.COLOR_WHITE,
    content
  }

  self.movable = MovableContainer:new {
    self[1],
    unmovable = self.unmovable,
  }
  scrollable.show_parent = self


  UIManager:setDirty(self, "ui")
end

--- @private
function Settings:onClose()
  UIManager:close(self)
  if self.on_return_callback then
    self.on_return_callback()
  end
end

--- @private
function Settings:onReturn()
  self:onClose()
end

--- @private
function Settings:updateSetting(key, value)
  -- fallback control ram_storage_enabled, ram_storage_size_mb
  if key == 'ram_storage_enabled' or key == 'ram_storage_size_mb' then
    local enabled = key == 'ram_storage_enabled' and value or self.settings.ram_storage_enabled
    local ram_storage_size_mb = key == 'ram_storage_size_mb' and value or self.settings.ram_storage_size_mb

    local response = Backend.mountFS({
      enabled = enabled,
      ram_storage_size_mb = ram_storage_size_mb,
    })

    if response.type == 'ERROR' then
      if key == 'ram_storage_enabled' then
        self.settings.ram_storage_enabled = false
      end
      ErrorDialog:show(response.message)
      return false
    end

    self.settings[key] = value

    return
  end

  self.settings[key] = value

  local response = Backend.setSettings(self.settings)
  if response.type == 'ERROR' then
    ErrorDialog:show(response.message)
    return
  end

  if key == "enabled_cron_check_mangas_update" or key == "source_skip_cron" then
    UIManager:show(InfoMessage:new {
      text = "You'll need to restart the app for this change to take effect"
    })
  end
end

--- @private
function Settings:_loginTracker(tracker)
  local loading = require("LoadingDialog"):new{message = _("Getting auth URL...")}
  loading:show()

  local resp = Backend.getTrackerAuthUrl(tracker)
  UIManager:close(loading)

  if resp.type == "ERROR" then
    ErrorDialog:show(resp.message)
    return
  end

  local auth_url = resp.body.url
  local tracker_name = tracker == "anilist" and "AniList" or "MyAnimeList"
  local input_hint = tracker == "anilist" and _("Paste access token") or _("Paste authorization code")

  local input_dialog
  input_dialog = require("ui/widget/inputdialog"):new{
    title = T(_("Log in to %1"), tracker_name),
    input_hint = input_hint,
    description = T(_("Open this URL in a browser, authorize, then paste the token or code:\n\n%1"), auth_url),
    input_type = "string",
    buttons = {
      {
        {
          text = _("Cancel"),
          callback = function()
            UIManager:close(input_dialog)
          end,
        },
        {
          text = _("Log in"),
          is_enter_default = true,
          callback = function()
            local token = input_dialog:getInputText()
            if token == "" then
              return
            end
            UIManager:close(input_dialog)

            local submit_loading = require("LoadingDialog"):new{message = _("Logging in...")}
            submit_loading:show()

            local body = {}
            if tracker == "anilist" then
              body.token = token
            else
              body.code = token
              body.state = resp.body.qr_id
            end

            local submit_resp = Backend.submitTrackerAuth(tracker, body)
            UIManager:close(submit_loading)

            if submit_resp.type == "ERROR" then
              ErrorDialog:show(submit_resp.message)
            else
              UIManager:show(InfoMessage:new{
                text = _("Logged in successfully"),
                timeout = 2,
              })
              -- Refresh the settings page.
              UIManager:close(self)
              Settings:fetchAndShow(self.on_return_callback)
            end
          end,
        },
      },
    },
  }
  UIManager:show(input_dialog)
  input_dialog:onShowKeyboard()
end

--- @private
function Settings:_logoutTracker(tracker)
  local confirm = require("ui/widget/confirmbox")
  local tracker_name = tracker == "anilist" and "AniList" or "MyAnimeList"
  UIManager:show(confirm:new{
    text = T(_("Log out of %1?"), tracker_name),
    ok_text = _("Log out"),
    ok_callback = function()
      local resp = Backend.clearTrackerAuth(tracker)
      if resp.type == "ERROR" then
        ErrorDialog:show(resp.message)
      else
        UIManager:show(InfoMessage:new{
          text = _("Logged out successfully"),
          timeout = 2,
        })
        -- Refresh the settings page.
        UIManager:close(self)
        Settings:fetchAndShow(self.on_return_callback)
      end
    end,
  })
end


function Settings:fetchAndShow(on_return_callback)
  local response = Backend.getSettings()
  if response.type == 'ERROR' then
    ErrorDialog:show(response.message)
    return
  end

  -- Fetch tracker services status for the tracking section.
  local tracker_services = nil
  local services_resp = Backend.getTrackerServices()
  if services_resp.type == "SUCCESS" then
    tracker_services = services_resp.body
  end

  local ui = Settings:new {
    settings = response.body,
    tracker_services = tracker_services,
    on_return_callback = on_return_callback
  }
  ui.on_return_callback = on_return_callback
  UIManager:show(ui)
end


return Settings

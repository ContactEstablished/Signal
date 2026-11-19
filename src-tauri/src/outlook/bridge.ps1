# Signal's read-only bridge to classic Outlook on this computer.
# Input: $env:SIGNAL_OUTLOOK_OP and JSON in $env:SIGNAL_OUTLOOK_ARGS. Output: one JSON line.
# It only reads, except 'open', which shows one message in Outlook.
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = New-Object Text.UTF8Encoding $false
function Iso($d) { $d.ToString('yyyy-MM-ddTHH:mm:ss.000Z') }
function Cut($s, $n) { if ($null -eq $s) { '' } elseif ($s.Length -gt $n) { $s.Substring(0, $n) } else { $s } }
function Store($ns, $id) { $s = $ns.GetStoreFromID($id); if ($null -eq $s) { throw 'This Outlook account is no longer in Outlook.' }; $s }
# Addresses in To (not Cc), so Signal can tell mail sent to the person directly.
function ToAddresses($m) {
  @(foreach ($r in $m.Recipients) {
    if ($r.Type -ne 1) { continue }
    $x = ''; try { $x = $r.PropertyAccessor.GetProperty('http://schemas.microsoft.com/mapi/proptag/0x39FE001F') } catch {}
    if (-not $x) { $x = $r.Address }
    if ($x) { [string]$x }
  }) -join ', '
}
function Smtp($m) {
  try { if ($m.SenderEmailType -eq 'EX') { $u = $m.Sender.GetExchangeUser(); if ($u) { return $u.PrimarySmtpAddress } } } catch {}
  $m.SenderEmailAddress
}
try {
  $a = $env:SIGNAL_OUTLOOK_ARGS | ConvertFrom-Json
  $ns = (New-Object -ComObject Outlook.Application).GetNamespace('MAPI')
  $out = @{}
  switch ($env:SIGNAL_OUTLOOK_OP) {
    'accounts' {
      $out.accounts = @(foreach ($x in $ns.Accounts) {
        $s = $null; try { $s = $x.DeliveryStore } catch {}
        if ($s) { @{ email = [string]$x.SmtpAddress; store_id = [string]$s.StoreID } }
      })
    }
    'events' {
      $from = ([DateTime]::Parse($a.from)).ToLocalTime().ToString('g')
      $to = ([DateTime]::Parse($a.to)).ToLocalTime().ToString('g')
      $out.results = @(foreach ($id in $a.stores) {
        try {
          $items = (Store $ns $id).GetDefaultFolder(9).Items
          $items.IncludeRecurrences = $true
          $items.Sort('[Start]')
          $r = $items.Restrict("[Start] < '$to' AND [End] > '$from'")
          $list = New-Object Collections.ArrayList
          $e = $r.GetFirst()
          while ($null -ne $e) {
            if ($list.Count -ge 5000) { throw 'This calendar has too many events to read. Signal reads up to 5,000 at a time.' }
            if ($e.Class -eq 26) {
              [void]$list.Add(@{ gid = [string]$e.GlobalAppointmentID; entry_id = [string]$e.EntryID; recurring = ($e.RecurrenceState -ne 0)
                start = (Iso $e.StartUTC); end = (Iso $e.EndUTC); all_day = [bool]$e.AllDayEvent; busy = [int]$e.BusyStatus
                response = [int]$e.ResponseStatus; meeting = [int]$e.MeetingStatus; subject = (Cut $e.Subject 300)
                location = (Cut $e.Location 1000); body = (Cut $e.Body 8000) })
            }
            $e = $r.GetNext()
          }
          @{ store_id = $id; events = $list }
        } catch { @{ store_id = $id; error = $_.Exception.Message } }
      })
    }
    'mail_list' {
      $since = ([DateTime]::Parse($a.since)).ToLocalTime().ToString('g')
      $out.results = @(foreach ($id in $a.stores) {
        try {
          $items = (Store $ns $id).GetDefaultFolder(6).Items
          $items.Sort('[ReceivedTime]', $true)
          $r = $items.Restrict("[ReceivedTime] >= '$since'")
          $list = New-Object Collections.ArrayList
          $m = $r.GetFirst()
          while ($null -ne $m -and $list.Count -lt 200) {
            if ($m.Class -eq 43) { [void]$list.Add([string]$m.EntryID) }
            $m = $r.GetNext()
          }
          @{ store_id = $id; ids = $list }
        } catch { @{ store_id = $id; error = $_.Exception.Message } }
      })
    }
    'mail_get' {
      $out.results = @(foreach ($q in $a.items) {
        try {
          $m = $ns.GetItemFromID($q.id, $q.store_id)
          $mid = ''; try { $mid = $m.PropertyAccessor.GetProperty('http://schemas.microsoft.com/mapi/proptag/0x1035001F') } catch {}
          @{ id = $q.id; store_id = $q.store_id; from_name = (Cut $m.SenderName 300); from_address = (Cut (Smtp $m) 300); to = (Cut $m.To 500); to_addresses = (Cut (ToAddresses $m) 1000)
            subject = (Cut $m.Subject 500); received = (Iso $m.ReceivedTime.ToUniversalTime()); message_id = (Cut $mid 500); body = (Cut $m.Body 20000) }
        } catch { @{ id = $q.id; store_id = $q.store_id; error = $_.Exception.Message } }
      })
    }
    'open' { $ns.GetItemFromID($a.id, $a.store_id).Display(); $out.opened = $true }
    default { throw 'Unknown request.' }
  }
  $out | ConvertTo-Json -Depth 6 -Compress
} catch {
  @{ error = $_.Exception.Message } | ConvertTo-Json -Compress
}

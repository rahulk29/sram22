proc doService {sock msg} {
  if {[string trim $msg] != ""} {
    puts "evaluating $msg"
    set eresult [eval $msg]
    # set eresult [eval "$msg"]
    puts "got $eresult"
    puts $sock "$eresult"
    flush $sock
  }
}

proc  svcHandler {sock} {
  set l [gets $sock]    ;# get the client packet
  if {[eof $sock]} {    ;# client gone or finished
     close $sock        ;# release the servers client channel
  } else {
    doService $sock $l
  }
}

proc accept {sock addr port} {
  
  fileevent $sock readable [list svcHandler $sock]

  puts "Accept from [fconfigure $sock -peername]"

  fconfigure $sock -buffering line -blocking 0

  puts "Accepted connection from $addr at [exec date]"
}


puts "Initializing socket on port $svcPort"

socket -server accept $svcPort
vwait events

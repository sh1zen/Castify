$files = 0;
dir . -include *.rs -Recurse | % {

$files++;

}

$i=0;
$k=0;
$words=0;

dir . -include *.rs -Recurse | % {

$count = (gc $_).count; 

    if ($count) {
        if($_.Extension -eq ".rs"){$rs = $rs + [int]$count;}
    } 

    $k =  $i/$files*100;
    $k = "{0:N0}" -f $k;

    $words = ((gc $_) | Measure-Object -word).Words + $words ;

    Write-Progress -Activity "Counting lines number:" -status "File $i completed $k%" -percentComplete ($k)
    $i++;
}

 write-host "`nTotal:";
 write-host "______________________________";
 write-host "`n  Files:  $files";
 write-host "  --------------------------";
 write-host "`n  Rust  :  $rs";
 write-host "______________________________";

 $tot = $rs;
 write-host "`n`nTotal lines: $tot";
  write-host "`nTotal words: $words`n`n";

pause
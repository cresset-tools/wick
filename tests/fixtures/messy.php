<?php
namespace App;
use App\Zebra;
use App\Apple;
class Foo {
    public function bar($x) {
        $name = "hi";
        $msg = "Hello ".$name;
        if($x){return "yes";}else{return "no";}
    }
}

// ============================================================
// Spool Sentry — one-spool platform + force path (issue #3)
// Editable OpenSCAD source. Review-only geometry: NOTHING here
// has been printed, fitted, or load-tested. See
// docs/verification/issue-3-pcb.md ("Mechanical evidence").
//
// Force path (per docs/architecture.md "Mechanical force path"):
//   spool -> removable platform -> platform bracket -> FIXED end
//   of a replaceable >=5 kg bar load cell -> base plate.
// The electronics carrier tray bolts to the base plate and is
// mechanically isolated from the moving load path.
//
// Safety boundary: USB 5 V SELV, observation only. No actuation,
// no mains, no safety claim, no certified measurement.
// ============================================================

// ---- Load cell envelope (PARAMETRIC — no MPN is frozen) ----
// The BOM carries a "replaceable >=5 kg bar load cell" with geometry
// finalized only when an exact part is selected (issue #6). These
// defaults are an ASSUMED generic bar-cell envelope and MUST be set
// from the chosen cell's datasheet drawing before fabrication.
cell_length   = 90;    // [mm] bar length  (ASSUMED)
cell_width    = 30;    // [mm] bar width   (ASSUMED)
cell_height   = 11;    // [mm] bar height  (ASSUMED)
cell_hole_pitch = 66;  // [mm] mounting-hole pitch (ASSUMED)
cell_hole_d   = 6.6;   // [mm] M5 clearance hole   (ASSUMED)
cell_bolt_d   = 5.0;   // [mm] M5 bolt
rated_kg      = 5;     // datasheet rated capacity used for stop design

// ---- Working / overload envelope (MECH-001, MEAS-002) ----
working_kg    = 2.5;   // centered static working load
deflection_mm = 1.0;   // expected tip deflection at working load (ASSUMED; bench item)
stop_gap_mm   = 3.0;   // hard-stop gap above working deflection; overload
                       // transfers through the stopposts, not the cell.
                       // Stop design is a bench-verification item before
                       // any publication claim.

// ---- Spool fit envelope (MECH-004) ----
// Envelope for "common" filament spools only — this is NOT a universal
// fit claim. Typical market envelope ~Ø205 x 70 mm max.
spool_od      = 205;   // [mm] envelope outer diameter
spool_width   = 70;    // [mm] envelope width
hub_bore      = 55;    // [mm] centering spigot (common hub bore; not universal)

// ---- Carrier PCB envelope (matches hardware/spool-sentry.kicad_pcb) ----
pcb_w = 66;  // board X
pcb_l = 75;  // board Y
pcb_t = 1.6;
m3_d  = 3.2;
// M3 hole centers on the board (mm, board origin at corner):
pcb_holes = [[3.6,3.6],[62.4,3.6],[62.4,71.4],[3.6,48.0]];

// ---- Generic helpers ----------------------------------------
module clear_hole(d, h) { cylinder(h=h, d=d, $fn=48); }

// ============================================================
// 1) Base plate — the fixed half of the force path.
//    Load cell bolts down at one end; carrier tray bolts flat.
// ============================================================
module base_plate() {
    L = 260; W = 140; t = 6;
    difference() {
        union() {
            translate([-W/2, 0, -t]) cube([W, L, t], center=false);
            // load-cell mounting pads at the fixed end
            for (dx = [-cell_hole_pitch/2, cell_hole_pitch/2])
                translate([dx, 18, -t])
                    linear_extrude(t)
                        offset(r=6) circle(d=cell_hole_d, $fn=32);
            // carrier-tray standoff bosses (M3), pattern mirrors the PCB
            for (h = pcb_holes)
                translate([h[0]-pcb_w/2, 40 + h[1], -t])
                    linear_extrude(6+8)
                        circle(d=8, $fn=32);
        }
        // bolt holes through cell pads
        for (dx = [-cell_hole_pitch/2, cell_hole_pitch/2])
            translate([dx, 18, -t-1]) rotate([90,0,0])
                clear_hole(cell_bolt_d, t+2);
        // M3 tapped holes in tray bosses (pilot 2.5 mm)
        for (h = pcb_holes)
            translate([h[0]-pcb_w/2, 40 + h[1], -t-1]) rotate([90,0,0])
                clear_hole(2.5, t+8+2);
        // cable slot for load-cell harness + USB exit (strain relief path)
        translate([-W/2-1, 96, -t-1]) cube([34, 22, t+2]);
    }
}

// ============================================================
// 2) Platform — the moving half. Removable (MECH-002): it simply
//    rests on the platform bracket; the load path is compression
//    through the bracket on the cell's fixed end.
// ============================================================
module platform() {
    pd = spool_od + 10;      // platform disc with 5 mm margin
    t  = 8;
    difference() {
        union() {
            cylinder(h=t, d=pd, $fn=96);
            // centering spigot for common hub bores
            translate([0,0,t]) cylinder(h=12, d=hub_bore, $fn=64);
            // load pad under spool rim/face
            translate([0,0,t]) cylinder(h=3, d=120, $fn=96);
        }
        // lightening pockets (keep rim stiff; bench item for stiffness)
        for (a = [0:90:270]) rotate([0,0,a])
            translate([pd/4, 0, 2]) cylinder(h=t-2, d=40, $fn=48);
    }
}

// ============================================================
// 3) Platform bracket + overload stop. Bracket transfers platform
//    load to the cell's free end; stopposts limit travel so an
//    overload path exists before the cell yields (bench item).
// ============================================================
module cell_bracket() {
    // U-bracket clamping the cell free end (bolted, hand tools)
    bw = cell_width + 12; bl = cell_length; bt = 6;
    difference() {
        union() {
            // floor over cell
            translate([-bw/2, 0, cell_height]) cube([bw, bl, bt], center=false);
            // side wings bolting to cell top holes
            for (dx = [-cell_hole_pitch/2, cell_hole_pitch/2])
                translate([dx-8, 18, 0]) cube([16, 16, cell_height+bt]);
        }
        // cell body clearance
        translate([-cell_width/2-0.5, 0, -1]) cube([cell_width+1, bl, cell_height+1]);
        for (dx = [-cell_hole_pitch/2, cell_hole_pitch/2])
            translate([dx, 18, cell_height+bt+1]) rotate([90,0,0])
                clear_hole(cell_bolt_d, cell_height+bt+2);
    }
}

module overload_stop_post(h) {
    // fixed post; platform/bracket top must clear this by `h`.
    // Gap = deflection_mm + stop_gap_mm above working position.
    difference() {
        cylinder(h=h, d=10, $fn=32);
        translate([0,0,h-4]) cylinder(h=5, d=4, $fn=24);
    }
}

// ============================================================
// 4) Carrier tray — mechanically ISOLATED from load path (per
//    architecture). Holds the 66x75 PCB on M3 standoffs; vents at
//    the SHT40 corner; USB + load-cell cables route down the slot.
// ============================================================
module carrier_tray() {
    wall = 2.4; floor_t = 2;
    x0 = -pcb_w/2 - wall; y0 = -wall; x1 = pcb_w/2 + wall; y1 = pcb_l + wall;
    difference() {
        union() {
            translate([x0, y0, 0]) cube([x1-x0, y1-y0, floor_t]);
            // walls (10 mm) on all sides
            translate([x0, y0, floor_t]) cube([x1-x0,  wall, 10]);
            translate([x0, y1-wall, floor_t]) cube([x1-x0, wall, 10]);
            translate([x0, y0, floor_t]) cube([wall, y1-y0, 10]);
            translate([x1-wall, y0, floor_t]) cube([wall, y1-y0, 10]);
        }
        // M3 standoff holes matching the board
        for (h = pcb_holes)
            translate([h[0]-pcb_w/2, h[1], -1])
                clear_hole(m3_d, floor_t+2);
        // USB port access at board edge (J1 at y≈68, x≈0 board coords)
        translate([-pcb_w/2-wall-1, 60, floor_t+3]) cube([wall+2, 16, 6]);
        // load-cell harness slot (J2 at y≈13, x≈0)
        translate([-pcb_w/2-wall-1, 6, floor_t+3]) cube([wall+2, 14, 6]);
        // vents at U3/SHT40 corner (board corner 62,44 -> tray coords)
        for (i = [0:4])
            translate([pcb_w/2-6, 30+i*6, -1]) rotate([0,90,0])
                clear_hole(3, wall*2+80);
    }
}

// ---- Sensor heat-separation note (MECH-003): U3 sits in the far
// corner of the board from U1/regulators (~38 mm on PCB; carrier
// tray vents on the same corner wall). Bring-up comparison in #6.

// ============================================================
// Assembly preview (F5) + individual STL exports (F6 into editor
// selection, or run scripts/export_mechanical_stl.sh).
// ============================================================
module show_assembly() {
    base_plate();
    translate([0, 0, 0])
        color([0.6,0.6,0.65]) cell_body_stub();
    translate([0, 0, cell_height]) cell_bracket();
    translate([0, 0, cell_height+6]) {
        translate([-40, 0, 0]) overload_stop_post(deflection_mm + stop_gap_mm);
        translate([ 40, 0, 0]) overload_stop_post(deflection_mm + stop_gap_mm);
    }
    translate([0, -cell_length-20, cell_height+6+deflection_mm+stop_gap_mm])
        color([0.9,0.7,0.2]) platform();
    translate([0, 40, 8]) color([0.2,0.5,0.2]) carrier_tray();
    %translate([0-pcb_w/2, 40, 8+2]) color([0,0.5,0])
        cube([pcb_w, pcb_l, pcb_t]);
    // spool fit envelope (review-only, ghosted)
    %translate([0, -cell_length-20, cell_height+6+deflection_mm+stop_gap_mm+11])
        cylinder(h=spool_width, d=spool_od, center=false);
}

module cell_body_stub() {
    // Placeholder bar envelope; replace with the selected cell
    // drawing in issue #6 (no MPN is frozen here).
    difference() {
        translate([-cell_width/2, 0, 0]) cube([cell_width, cell_length, cell_height]);
        for (dx = [-cell_hole_pitch/2, cell_hole_pitch/2])
            translate([dx, 18, -1]) clear_hole(cell_hole_d, cell_height+2);
    }
}

// ---- export switches: openscad -D export_part=... ----
export_part = "assembly";
if (export_part == "base")        base_plate();
else if (export_part == "platform") platform();
else if (export_part == "bracket") cell_bracket();
else if (export_part == "tray")   carrier_tray();
else if (export_part == "stop")   overload_stop_post(deflection_mm + stop_gap_mm);
else show_assembly();

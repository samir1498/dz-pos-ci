// Every kit component, in every state it has, on one page.
//
// It exists for three readers. A reviewer, who compares it against the
// mockups instead of clicking through nine screens. The theme work, because
// four themes times a dozen components is a lot of places for a role to be
// wrong and this is the one page where all of them are visible at once. And
// the Claude Design project, whose preview cards are built from this markup
// so the design tool and the app cannot drift.
//
// It is not in the navigation and it is not in a shipped build: the route in
// routes/kit.tsx refuses outside dev.
//
// The labels here are component names and sample data, not product copy, so
// they are written literally rather than through i18n. The components' own
// strings still come from the dictionaries: a `StatusPill` says what the
// running language says, and this page proves that by rendering them.

import {
  Ellipsis,
  Package,
  Printer,
  Trash2,
} from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { Keypad, keyedAmount } from "@/components/Keypad";
import { Money } from "@/components/Money";
import { MoneyInput } from "@/components/MoneyInput";
import { PageHeader } from "@/components/PageHeader";
import { PayButton } from "@/components/PayButton";
import { ProductTile } from "@/components/ProductTile";
import { StatusPill, type Status } from "@/components/StatusPill";
import { Badge } from "@/components/ui/badge";
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "@/components/ui/breadcrumb";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle, SheetTrigger } from "@/components/ui/sheet";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useTranslation } from "@/i18n";

/** One block of the page: a name, and the states of one component under it. */
function Section({ name, children }: { name: string; children: React.ReactNode }) {
  return (
    <section data-testid={`kit-${name.toLowerCase().replace(/\s+/g, "-")}`} className="space-y-3">
      <h3 className="text-sm font-semibold tracking-wide text-muted-foreground uppercase">{name}</h3>
      <div className="flex flex-wrap items-center gap-3 rounded-lg border border-border bg-card p-4">
        {children}
      </div>
    </section>
  );
}

interface Line {
  readonly id: number;
  readonly product: string;
  readonly qty: number;
  readonly total: number;
  readonly status: Status;
}

const LINES: readonly Line[] = [
  { id: 1, product: "Farine 5 kg", qty: 12, total: 1_284_000, status: "paid" },
  { id: 2, product: "Sucre 1 kg", qty: 3, total: 27_000, status: "open" },
  { id: 3, product: "Huile 5 L", qty: 1, total: 129_950, status: "low" },
];

const COLUMNS: readonly Column<Line>[] = [
  { id: "product", header: "Produit", cell: (line) => line.product },
  { id: "qty", header: "Qté", numeric: true, cell: (line) => line.qty },
  { id: "status", header: "État", cell: (line) => <StatusPill status={line.status} /> },
  { id: "total", header: "Total", money: true, cell: (line) => <Money centimes={line.total} /> },
];

const STATES: readonly Status[] = ["issued", "cancelled", "paid", "open", "low"];

export function KitPage() {
  const { dir } = useTranslation();
  const [amount, setAmount] = useState<number | null>(1_284_000);
  const [typedAmount, setTypedAmount] = useState<number | null>(150_000);
  const [checked, setChecked] = useState(true);
  const [on, setOn] = useState(true);

  return (
    <div data-testid="kit-page" className="space-y-8 pb-16">
      <PageHeader
        title="Le kit"
        description="Chaque composant, dans chacun de ses états. Change de thème et de langue dans la barre du haut."
        actions={
          <>
            <Button variant="outline">
              <Icon as={Printer} size={18} />
              Imprimer
            </Button>
            <PayButton>Encaisser</PayButton>
          </>
        }
      />

      <Section name="Button">
        <Button>Défaut</Button>
        <Button variant="secondary">Secondaire</Button>
        <Button variant="outline">Contour</Button>
        <Button variant="ghost">Discret</Button>
        <Button variant="link">Lien</Button>
        <Button variant="destructive">Supprimer</Button>
        <Button disabled>Désactivé</Button>
        <Button size="sm">Petit</Button>
        <Button size="lg">Grand</Button>
        <Button size="icon" aria-label="Options">
          <Icon as={Ellipsis} size={18} />
        </Button>
        <PayButton>Encaisser</PayButton>
        <PayButton disabled>Encaisser</PayButton>
      </Section>

      <Section name="StatusPill">
        {STATES.map((status) => (
          <StatusPill key={status} status={status} />
        ))}
      </Section>

      <Section name="Badge">
        <Badge>Défaut</Badge>
        <Badge variant="secondary">Secondaire</Badge>
        <Badge variant="outline">Contour</Badge>
        <Badge variant="destructive">Refusé</Badge>
      </Section>

      <Section name="Money">
        <Money centimes={0} />
        <Money centimes={12_345} />
        <Money centimes={1_284_000} />
        <Money centimes={-98_750} />
      </Section>

      <Section name="Fields">
        <div className="grid w-full gap-4 sm:grid-cols-2">
          <FormField label="Nom" required hint="Tel qu'il apparaît sur la facture.">
            {(parts) => <Input {...parts} placeholder="Superette El Baraka" />}
          </FormField>
          <FormField label="Téléphone" error="Numéro incomplet.">
            {(parts) => <Input {...parts} defaultValue="0770 11" />}
          </FormField>
          <FormField label="Montant" hint="Vide veut dire « pas de montant ».">
            {(parts) => <MoneyInput {...parts} value={amount} onChange={setAmount} />}
          </FormField>
          <FormField label="Taux de TVA">
            {(parts) => (
              <Select defaultValue="1900">
                <SelectTrigger id={parts.id} className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="0">0 %</SelectItem>
                  <SelectItem value="900">9 %</SelectItem>
                  <SelectItem value="1900">19 %</SelectItem>
                </SelectContent>
              </Select>
            )}
          </FormField>
          <FormField label="Notes" className="sm:col-span-2">
            {(parts) => <Textarea {...parts} rows={3} placeholder="Ce que le comptoir doit savoir." />}
          </FormField>
          <FormField label="Désactivé">{(parts) => <Input {...parts} disabled value="" readOnly />}</FormField>
        </div>
      </Section>

      {/* The pad and the box it types into, because a keypad on its own says
          nothing: what is worth looking at is the amount growing in the
          figure face as the keys go down. */}
      <Section name="Keypad">
        <div className="w-full max-w-xs space-y-3">
          <div className="rounded-md border border-border bg-muted px-3 py-2 text-end">
            <Money centimes={typedAmount ?? 0} className="text-xl" />
          </div>
          <Keypad onKey={(key) => setTypedAmount((current) => keyedAmount(current, key))} />
        </div>
      </Section>

      <Section name="Toggles">
        <Label className="flex items-center gap-2">
          <Checkbox checked={checked} onCheckedChange={(next) => setChecked(next === true)} />
          Actif au comptoir
        </Label>
        <Label className="flex items-center gap-2">
          <Switch checked={on} onCheckedChange={setOn} />
          Imprimer le ticket
        </Label>
        <Label className="flex items-center gap-2 opacity-50">
          <Checkbox disabled />
          Désactivé
        </Label>
      </Section>

      <Section name="Tabs">
        <Tabs defaultValue="lines" className="w-full">
          <TabsList>
            <TabsTrigger value="lines">Lignes</TabsTrigger>
            <TabsTrigger value="payments">Règlements</TabsTrigger>
            <TabsTrigger value="notes" disabled>
              Notes
            </TabsTrigger>
          </TabsList>
          <TabsContent value="lines" className="pt-3 text-sm text-muted-foreground">
            Les lignes du document.
          </TabsContent>
          <TabsContent value="payments" className="pt-3 text-sm text-muted-foreground">
            Ce qui a été encaissé.
          </TabsContent>
        </Tabs>
      </Section>

      <Section name="Card">
        <Card className="w-full max-w-sm">
          <CardHeader>
            <CardTitle>Caisse du jour</CardTitle>
            <CardDescription>Depuis l'ouverture.</CardDescription>
          </CardHeader>
          <CardContent className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">Encaissé</span>
            <Money centimes={1_284_000} className="text-lg" />
          </CardContent>
        </Card>
      </Section>

      <Section name="Overlays">
        <Dialog>
          <DialogTrigger asChild>
            <Button variant="outline" data-testid="kit-dialog-trigger">
              Ouvrir un dialogue
            </Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Annuler la facture ?</DialogTitle>
              <DialogDescription>
                Une facture émise ne s'efface pas. Elle sera annulée par un avoir.
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="ghost">Revenir</Button>
              <Button variant="destructive">
                <Icon as={Trash2} size={18} />
                Annuler
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        <Sheet>
          <SheetTrigger asChild>
            <Button variant="outline" data-testid="kit-sheet-trigger">
              Ouvrir un panneau
            </Button>
          </SheetTrigger>
          {/* The side is physical on purpose (the panel's edge, its border
              and the half it slides in from have to agree), so the caller
              picks it from the page direction the way AppShell does. */}
          <SheetContent side={dir === "rtl" ? "left" : "right"}>
            <SheetHeader>
              <SheetTitle>Le client</SheetTitle>
              <SheetDescription>Le panneau s'ouvre du côté opposé à la lecture.</SheetDescription>
            </SheetHeader>
          </SheetContent>
        </Sheet>

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="outline" data-testid="kit-menu-trigger">
              Actions
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent>
            <DropdownMenuItem>Ouvrir la fiche</DropdownMenuItem>
            <DropdownMenuItem>Imprimer</DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem variant="destructive">Supprimer</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" data-testid="kit-tooltip-trigger">
              Survoler
            </Button>
          </TooltipTrigger>
          <TooltipContent>Le timbre s'applique aux règlements en espèces.</TooltipContent>
        </Tooltip>

        <Button
          variant="outline"
          data-testid="kit-toast-trigger"
          onClick={() => toast.success("Facture enregistrée.")}
        >
          Afficher un message
        </Button>
      </Section>

      <Section name="Breadcrumb">
        <Breadcrumb>
          <BreadcrumbList>
            <BreadcrumbItem>
              <BreadcrumbLink href="#">Clients</BreadcrumbLink>
            </BreadcrumbItem>
            <BreadcrumbSeparator />
            <BreadcrumbItem>
              <BreadcrumbPage>Entreprise Benali</BreadcrumbPage>
            </BreadcrumbItem>
          </BreadcrumbList>
        </Breadcrumb>
      </Section>

      <Section name="Separator and Skeleton">
        <div className="w-full space-y-3">
          <Separator />
          <div className="space-y-2">
            <Skeleton className="h-4 w-1/3" />
            <Skeleton className="h-4 w-2/3" />
            <Skeleton className="h-4 w-1/2" />
          </div>
        </div>
      </Section>

      <Section name="ScrollArea">
        <ScrollArea className="h-32 w-full rounded-md border border-border p-3">
          <div className="space-y-2 text-sm">
            {Array.from({ length: 12 }, (_, index) => (
              <p key={index}>Ligne de ticket numéro {index + 1}</p>
            ))}
          </div>
        </ScrollArea>
      </Section>

      <Section name="ProductTile">
        <div
          data-testid="kit-tiles"
          className="grid w-full gap-3 sm:grid-cols-3 lg:grid-cols-4"
        >
          <ProductTile name="Farine 5 kg" priceCentimes={128_400} qtyMilli={42_000} lowStockAtMilli={5_000} />
          <ProductTile name="Sucre cristallisé 1 kg" priceCentimes={27_000} qtyMilli={4_000} lowStockAtMilli={5_000} />
          <ProductTile name="Huile de table 5 L" priceCentimes={129_950} qtyMilli={0} lowStockAtMilli={5_000} />
          <ProductTile
            name="Café moulu 250 g"
            priceCentimes={45_000}
            qtyMilli={0}
            lowStockAtMilli={2_000}
            disabled
          />
        </div>
      </Section>

      <Section name="DataTable">
        <div className="w-full space-y-4">
          <DataTable
            columns={COLUMNS}
            rows={LINES}
            rowKey={(line) => line.id}
            caption="Les lignes du ticket"
            data-testid="kit-table"
            actions={() => (
              <Button variant="ghost" size="icon" aria-label="Actions de la ligne">
                <Icon as={Ellipsis} size={18} />
              </Button>
            )}
          />
          <DataTable
            columns={COLUMNS}
            rows={[]}
            rowKey={(line) => line.id}
            caption="Une liste vide"
            empty={
              <EmptyState
                icon={Package}
                title="Aucun produit"
                description="Importez un catalogue, ou ajoutez le premier à la main."
                action={<Button>Ajouter un produit</Button>}
              />
            }
          />
        </div>
      </Section>
    </div>
  );
}
